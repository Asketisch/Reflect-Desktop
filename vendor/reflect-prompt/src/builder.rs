//! `builder` — `LayeredPrompt` composition + `PromptBuilder`.
//!
//! Reflect's `prompt_builder.py` composes a system prompt from three layers:
//! - **core** = agent's `system_prompt` (Markdown body of `agent.md`) +
//!   memory injection (capped 8KB). This is the **stable, cacheable** layer.
//! - **append** = mid-session appends (e.g. tool list summary). Currently
//!   unused; reserved for future M5/M6 use.
//! - **ephemeral** = `<system-reminder>` block injected as a `User` message
//!   (not cacheable) carrying the live tool list, skills catalog, and
//!   per-turn reminders (iteration count, workspace path).
//!
//! `PromptBuilder` owns a [`CacheBreakDetector`] and provides a single
//! `build_request` helper that turns `(messages, tools)` into a
//! `ChatRequest` with cache_control already injected.

use reflect_llm::{
    Capabilities, ChatMessage, ChatRequest, ContentBlock, SystemBlock, SystemBlocks, ToolSpec,
    UserContent,
};

use crate::caching::CacheBreakDetector;

/// Header that marks the memory injection section in the core system prompt.
pub const MEMORY_HEADER: &str = "## Agent Memory";

/// Prefix that wraps the ephemeral block so the LLM recognizes it as
/// out-of-band metadata (not user-authored content).
pub const EPHEMERAL_PREFIX: &str = "<system-reminder>";

/// Prefix used for the core (cacheable) layer in diagnostics.
pub const CORE_PREFIX: &str = "## Core System Prompt";

/// Three-layer system prompt composition.
///
/// `core` is cacheable. `append` is reserved. `ephemeral` is injected as a
/// `User` message and never sent through the cache.
#[derive(Debug, Clone, Default)]
pub struct LayeredPrompt {
    /// Stable, cacheable base (agent system_prompt + memory).
    pub core: String,
    /// Optional mid-session append. Empty when unused.
    pub append: String,
    /// Per-turn ephemeral content (rendered into a User-role
    /// `<system-reminder>` block).
    pub ephemeral: String,
}

impl LayeredPrompt {
    /// New empty layered prompt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the core layer.
    pub fn with_core(mut self, core: impl Into<String>) -> Self {
        self.core = core.into();
        self
    }

    /// Set the append layer.
    pub fn with_append(mut self, append: impl Into<String>) -> Self {
        self.append = append.into();
        self
    }

    /// Set the ephemeral layer.
    pub fn with_ephemeral(mut self, ephemeral: impl Into<String>) -> Self {
        self.ephemeral = ephemeral.into();
        self
    }

    /// Compose the core layer from an agent's system_prompt + a memory
    /// injection. Memory is appended under `MEMORY_HEADER`; if `memory` is
    /// empty the header is omitted entirely.
    pub fn compose_core(agent_system_prompt: &str, memory: &str) -> String {
        let mut out = String::with_capacity(agent_system_prompt.len() + memory.len() + 64);
        out.push_str(agent_system_prompt);
        if !memory.trim().is_empty() {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            out.push_str(MEMORY_HEADER);
            out.push('\n');
            out.push_str(memory.trim());
            out.push('\n');
        }
        out
    }

    /// Compose the ephemeral block from tool specs, skills catalog, and a
    /// free-form reminder line (e.g. iteration count).
    pub fn compose_ephemeral(tools: &[ToolSpec], skills_catalog: &str, reminder: &str) -> String {
        let mut out = String::with_capacity(256);
        out.push_str(EPHEMERAL_PREFIX);
        out.push('\n');
        // Active tools.
        out.push_str("## Active Tools\n");
        if tools.is_empty() {
            out.push_str("(none)\n");
        } else {
            for t in tools {
                let ToolSpec::Function { name, .. } = t;
                out.push_str("- ");
                out.push_str(&name);
                out.push('\n');
            }
        }
        out.push('\n');
        // Skills catalog.
        if !skills_catalog.trim().is_empty() {
            out.push_str("## Skills\n");
            out.push_str(skills_catalog.trim());
            out.push('\n');
            out.push('\n');
        }
        // Reminder.
        if !reminder.trim().is_empty() {
            out.push_str("## Reminder\n");
            out.push_str(reminder.trim());
            out.push('\n');
        }
        out.push_str("</system-reminder>");
        out
    }
}

/// Owns a `CacheBreakDetector` and produces `ChatRequest`s with cache
/// breakpoints already injected. Stateless aside from the detector.
#[derive(Debug, Default)]
pub struct PromptBuilder {
    detector: CacheBreakDetector,
    /// v1.1.0 Phase 4:用户追加到 `core` 之后的命名 section 列表。
    /// `build_request` 时按插入顺序拼到 `core` 末尾(在 `append` 之前),
    /// 便于 coordinator prompt 等按需注入而不污染主 cacheable layer。
    /// 空 name 或空 body 在 `build_request` 跳过。
    sections: Vec<(String, String)>,
}

impl PromptBuilder {
    /// New builder with empty detector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a pre-existing detector.
    pub fn with_detector(detector: CacheBreakDetector) -> Self {
        Self {
            detector,
            sections: Vec::new(),
        }
    }

    /// Most recent observed request hash, or `None` if `build_request` has
    /// never been called.
    pub fn last_hash(&self) -> Option<&str> {
        self.detector.last_hash()
    }

    /// Reset the detector so the next `build_request` reports a change.
    pub fn reset_detector(&mut self) {
        self.detector.reset();
    }

    /// v1.1.0 Phase 4:追加一段命名 prompt section。
    ///
    /// `build_request` 时把 `(name, body)` 按追加顺序拼到 `core` 末尾
    /// (在 [`LayeredPrompt::append`] 之前),格式:
    ///
    /// ```text
    /// <core>
    ///
    /// ## <name>
    /// <body>
    /// ```
    ///
    /// 空 `name` 或空 `body` 被静默忽略 —— 避免空 section 触发 cache miss。
    /// `name` 重复时按调用顺序全部保留(coordinator 多 section 时合理)。
    pub fn add_section(&mut self, name: impl Into<String>, body: impl Into<String>) {
        let name = name.into();
        let body = body.into();
        if name.trim().is_empty() || body.trim().is_empty() {
            return;
        }
        self.sections.push((name, body));
    }

    /// 当前已注册的 section 数量(测试 / 诊断用)。
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// 按名称 upsert section:同名则替换 body,否则追加。
    /// 供 coordinator 热重载更新 `Coordinator` 段,避免重复插入。
    pub fn upsert_section(&mut self, name: impl Into<String>, body: impl Into<String>) {
        let name = name.into();
        let body = body.into();
        if name.trim().is_empty() || body.trim().is_empty() {
            return;
        }
        if let Some(entry) = self.sections.iter_mut().find(|(n, _)| *n == name) {
            entry.1 = body;
        } else {
            self.sections.push((name, body));
        }
    }

    /// 移除指定名称的 section(不存在时 no-op)。
    pub fn remove_section(&mut self, name: &str) {
        self.sections.retain(|(n, _)| n != name);
    }

    /// Build a `ChatRequest` from the layer set, the live messages, and the
    /// tool list. Returns the request and a `bool` indicating whether the
    /// cache fingerprint changed (Anthropic's cache will miss on change).
    ///
    /// If `caps.prompt_caching` is false the request is returned unchanged
    /// (no cache_control injection).
    ///
    /// v1.1.0 Phase 4:若 `self.sections` 非空,把每段追加到 `core` 末尾
    /// (在 `append` 之前),然后走原 cache 注入路径。section 改动
    /// 同样会触发 cache fingerprint 变化,与 `core` 改写等价。
    pub fn build_request(
        &mut self,
        layers: &LayeredPrompt,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolSpec>,
        model: impl Into<String>,
        caps: &Capabilities,
    ) -> (ChatRequest, bool) {
        let layers = if self.sections.is_empty() {
            layers.clone()
        } else {
            compose_with_sections(layers, &self.sections)
        };
        let mut req = ChatRequest {
            model: model.into(),
            messages,
            tools,
            system: layer_to_system_blocks(&layers),
            ..Default::default()
        };
        // Try to inject cache control. Skip silently if unsupported.
        let injected = if caps.prompt_caching {
            crate::caching::inject_cache_control(&mut req, caps).is_ok()
        } else {
            false
        };
        let (changed, _hash) = self.detector.observe(&req);
        // `changed` is informational; the actual request is already correct.
        // Suppress unused warning when `injected` is false on OpenAI.
        let _ = injected;
        (req, changed)
    }
}

fn layer_to_system_blocks(layers: &LayeredPrompt) -> SystemBlocks {
    let mut blocks = Vec::new();
    if !layers.core.trim().is_empty() {
        blocks.push(SystemBlock {
            text: layers.core.clone(),
            cache_control: None,
            ephemeral: false,
        });
    }
    if !layers.append.trim().is_empty() {
        blocks.push(SystemBlock {
            text: layers.append.clone(),
            cache_control: None,
            ephemeral: false,
        });
    }
    SystemBlocks(blocks)
}

/// v1.1.0 Phase 4:把 `(name, body)` section 列表追加到 `core` 末尾,
/// 返回新 `LayeredPrompt`。原 `layers.append` 在 sections 之后追加,
/// 保持原有 priority(core → sections → append → ephemeral)。
fn compose_with_sections(layers: &LayeredPrompt, sections: &[(String, String)]) -> LayeredPrompt {
    let mut core = layers.core.clone();
    if !core.is_empty() && !core.ends_with('\n') {
        core.push('\n');
    }
    for (name, body) in sections {
        if name.trim().is_empty() || body.trim().is_empty() {
            continue;
        }
        core.push('\n');
        core.push_str("## ");
        core.push_str(name.trim());
        core.push('\n');
        core.push_str(body.trim());
        core.push('\n');
    }
    LayeredPrompt {
        core,
        append: layers.append.clone(),
        ephemeral: layers.ephemeral.clone(),
    }
}

/// Inject the ephemeral block as a trailing `User` message so it reaches
/// the model without polluting the cacheable system blocks.
pub fn inject_ephemeral_as_user(req: &mut ChatRequest, ephemeral: &str) {
    if ephemeral.trim().is_empty() {
        return;
    }
    req.messages.push(ChatMessage::User(UserContent {
        blocks: vec![ContentBlock::text(ephemeral)],
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::ToolSpec;
    use serde_json::json;

    #[test]
    fn compose_core_appends_memory_header() {
        let out = LayeredPrompt::compose_core("you are a reviewer", "## Facts\n- foo");
        assert!(out.contains("you are a reviewer"));
        assert!(out.contains(MEMORY_HEADER));
        assert!(out.contains("- foo"));
    }

    #[test]
    fn compose_core_omits_memory_header_when_empty() {
        let out = LayeredPrompt::compose_core("base", "");
        assert!(!out.contains(MEMORY_HEADER));
        assert_eq!(out, "base");
    }

    #[test]
    fn compose_core_omits_memory_header_when_whitespace_only() {
        let out = LayeredPrompt::compose_core("base", "   \n  \n");
        assert!(!out.contains(MEMORY_HEADER));
    }

    #[test]
    fn compose_ephemeral_renders_tools_skills_reminder() {
        let tools = vec![ToolSpec::Function {
            name: "bash".into(),
            description: "shell".into(),
            parameters: json!({}),
        }];
        let out = LayeredPrompt::compose_ephemeral(&tools, "## Catalog\n- foo", "iter 1/32");
        assert!(out.starts_with(EPHEMERAL_PREFIX));
        assert!(out.contains("- bash"));
        assert!(out.contains("## Catalog"));
        assert!(out.contains("## Reminder"));
        assert!(out.contains("iter 1/32"));
        assert!(out.ends_with("</system-reminder>"));
    }

    #[test]
    fn compose_ephemeral_handles_no_tools() {
        let out = LayeredPrompt::compose_ephemeral(&[], "", "x");
        assert!(out.contains("(none)"));
    }

    #[test]
    fn compose_ephemeral_handles_no_skills_no_reminder() {
        let out = LayeredPrompt::compose_ephemeral(&[], "", "");
        assert!(out.starts_with(EPHEMERAL_PREFIX));
        assert!(out.ends_with("</system-reminder>"));
    }

    #[test]
    fn layer_to_system_blocks_skips_empty_layers() {
        let l = LayeredPrompt {
            core: "core".into(),
            append: "".into(),
            ephemeral: "ignored".into(),
        };
        let blocks = layer_to_system_blocks(&l);
        assert_eq!(blocks.0.len(), 1);
        assert_eq!(blocks.0[0].text, "core");
    }

    #[test]
    fn inject_ephemeral_as_user_skips_empty() {
        let mut req = ChatRequest::default();
        inject_ephemeral_as_user(&mut req, "");
        assert!(req.messages.is_empty());
    }

    #[test]
    fn inject_ephemeral_as_user_appends_user_message() {
        let mut req = ChatRequest::default();
        inject_ephemeral_as_user(&mut req, "<system-reminder>x</system-reminder>");
        assert_eq!(req.messages.len(), 1);
        match &req.messages[0] {
            ChatMessage::User(u) => {
                assert_eq!(u.blocks.len(), 1);
            }
            other => panic!("expected User, got {other:?}"),
        }
    }

    #[test]
    fn build_request_sets_system_blocks_from_core() {
        let mut b = PromptBuilder::new();
        let layers = LayeredPrompt {
            core: "agent says".into(),
            append: "extra".into(),
            ephemeral: String::new(),
        };
        let caps = Capabilities::default();
        let (req, changed) = b.build_request(&layers, vec![], vec![], "m", &caps);
        assert_eq!(req.system.0.len(), 2);
        assert_eq!(req.system.0[0].text, "agent says");
        assert_eq!(req.system.0[1].text, "extra");
        assert!(changed, "first observation reports changed");
    }

    #[test]
    fn build_request_reports_change_on_second_call_when_layers_differ() {
        let mut b = PromptBuilder::new();
        let caps = Capabilities::default();
        let l1 = LayeredPrompt {
            core: "v1".into(),
            ..Default::default()
        };
        let (_, c1) = b.build_request(&l1, vec![], vec![], "m", &caps);
        assert!(c1);
        let l2 = LayeredPrompt {
            core: "v2".into(),
            ..Default::default()
        };
        let (_, c2) = b.build_request(&l2, vec![], vec![], "m", &caps);
        assert!(c2, "second call with different core must report changed");
    }

    #[test]
    fn build_request_does_not_inject_cache_control_without_capability() {
        let mut b = PromptBuilder::new();
        let layers = LayeredPrompt {
            core: "x".into(),
            ..Default::default()
        };
        let caps = Capabilities::default(); // no prompt_caching
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        assert!(req.system.0[0].cache_control.is_none());
        assert!(req.cache_control.is_empty());
    }

    #[test]
    fn build_request_injects_cache_control_with_capability() {
        let mut b = PromptBuilder::new();
        let layers = LayeredPrompt {
            core: "x".into(),
            ..Default::default()
        };
        let caps = Capabilities {
            prompt_caching: true,
            ..Capabilities::default()
        };
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        // Last (only) SystemBlock has cache_control.
        assert!(req.system.0[0].cache_control.is_some());
    }

    // ── v1.1.0 Phase 4: add_section ─────────────────────────────

    /// `add_section` 注册的 section 在 `build_request` 时拼到 core 末尾。
    #[test]
    fn add_section_appends_to_core_on_build() {
        let mut b = PromptBuilder::new();
        b.add_section("Coordinator", "Custom coordinator rules.");
        assert_eq!(b.section_count(), 1);
        let layers = LayeredPrompt {
            core: "base".into(),
            ..Default::default()
        };
        let caps = Capabilities::default();
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        // core + 1 section 注入后,SystemBlocks 应至少 1 个,合并文本含 section。
        assert!(!req.system.0.is_empty());
        let text = &req.system.0[0].text;
        assert!(text.contains("base"), "core 缺失: {text}");
        assert!(
            text.contains("## Coordinator"),
            "section header 缺失: {text}"
        );
        assert!(
            text.contains("Custom coordinator rules."),
            "section body 缺失: {text}"
        );
    }

    /// 多次 `add_section` 按插入顺序追加。
    #[test]
    fn add_section_preserves_order() {
        let mut b = PromptBuilder::new();
        b.add_section("First", "AAA");
        b.add_section("Second", "BBB");
        let layers = LayeredPrompt {
            core: "base".into(),
            ..Default::default()
        };
        let caps = Capabilities::default();
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        let text = &req.system.0[0].text;
        let first_pos = text.find("AAA").expect("First body 缺失");
        let second_pos = text.find("BBB").expect("Second body 缺失");
        assert!(first_pos < second_pos, "顺序错误:{text}");
    }

    /// 空 name 或空 body 被静默忽略,不增加 section_count。
    #[test]
    fn add_section_ignores_empty_inputs() {
        let mut b = PromptBuilder::new();
        b.add_section("", "body");
        b.add_section("name", "");
        b.add_section("   ", "  ");
        b.add_section("ok", "   \n  \n");
        assert_eq!(b.section_count(), 0, "全部空输入应被忽略");
    }

    /// `upsert_section` 同名替换,不重复追加。
    #[test]
    fn upsert_section_replaces_same_name() {
        let mut b = PromptBuilder::new();
        b.upsert_section("Coordinator", "v1");
        b.upsert_section("Coordinator", "v2");
        assert_eq!(b.section_count(), 1);
        let layers = LayeredPrompt {
            core: "base".into(),
            ..Default::default()
        };
        let caps = Capabilities::default();
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        assert!(req.system.0[0].text.contains("v2"));
        assert!(!req.system.0[0].text.contains("v1"));
    }

    /// `remove_section` 清掉指定段。
    #[test]
    fn remove_section_drops_named_entry() {
        let mut b = PromptBuilder::new();
        b.upsert_section("Coordinator", "rules");
        b.remove_section("Coordinator");
        assert_eq!(b.section_count(), 0);
    }

    /// 无 section 时 build_request 走原路径(`compose_with_sections` 不动 core)。
    #[test]
    fn build_request_without_sections_leaves_core_intact() {
        let mut b = PromptBuilder::new();
        let layers = LayeredPrompt {
            core: "unchanged".into(),
            ..Default::default()
        };
        let caps = Capabilities::default();
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        assert!(req.system.0[0].text.contains("unchanged"));
        assert!(!req.system.0[0].text.contains("## "));
    }

    /// append layer 仍在 sections 之后。
    #[test]
    fn add_section_does_not_consume_append_layer() {
        let mut b = PromptBuilder::new();
        b.add_section("Coord", "rules");
        let layers = LayeredPrompt {
            core: "core".into(),
            append: "appendix".into(),
            ..Default::default()
        };
        let caps = Capabilities::default();
        let (req, _) = b.build_request(&layers, vec![], vec![], "m", &caps);
        // core+section 合并到第 1 个 block,append 是独立第 2 个 block。
        assert_eq!(req.system.0.len(), 2);
        assert!(req.system.0[0].text.contains("core"));
        assert!(req.system.0[0].text.contains("## Coord"));
        assert!(req.system.0[1].text.contains("appendix"));
    }
}
