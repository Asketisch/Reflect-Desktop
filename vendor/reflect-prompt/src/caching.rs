//! `caching` — Anthropic `cache_control` injection + change detection.
//!
//! Mirrors reflect `prompt_caching.py`. Always attaches `cache_control:
//! Ephemeral` to the LAST `SystemBlock` (mandatory breakpoint). Optionally
//! attaches a "prefix anchor" breakpoint at a non-meta message (default
//! = 2nd-from-last non-meta message, `offset = -1`). Skips entirely for
//! providers without `prompt_caching` capability (OpenAI, Ollama).
//!
//! `CacheBreakDetector` keeps a sha1 of `(system, tools, model)`. When the
//! hash changes the caller knows a cache breakpoint needs to be
//! re-emitted; we don't push a separate protocol event for this (model_call
//! already streams the request — Anthropic's own response will reflect the
//! fresh cache miss).

use reflect_llm::{
    CacheControl, CacheControlKind, CacheTtl, Capabilities, ChatMessage, ChatRequest, SystemBlock,
    SystemBlocks,
};
use sha1::{Digest, Sha1};
use thiserror::Error;

/// Default offset for the prefix anchor breakpoint. -1 = last non-meta
/// message (Python-style negative indexing; -2 = 2nd from last; 0 = first).
pub const DEFAULT_PREFIX_ANCHOR_OFFSET: i32 = -1;

/// Errors from cache injection.
#[derive(Debug, Error)]
pub enum CacheBreakError {
    /// Provider does not support prompt caching; injection skipped.
    #[error("provider does not support prompt caching")]
    Unsupported,
}

/// 累计观测统计 —— v1+ cache 监控增强,供 TUI / metrics 读取。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CacheMonitorStats {
    /// `observe` 调用次数。
    pub observe_count: u64,
    /// fingerprint 变化次数。
    pub change_count: u64,
    /// 最近一次 hash。
    pub last_hash: Option<String>,
    /// 最近一次是否判定为 changed。
    pub last_changed: bool,
}

impl CacheMonitorStats {
    /// 命中率 = 1 - change_count/observe_count(首次 observe 计为 change)。
    pub fn hit_rate(&self) -> f64 {
        if self.observe_count == 0 {
            return 0.0;
        }
        let hits = self.observe_count.saturating_sub(self.change_count);
        hits as f64 / self.observe_count as f64
    }
}

/// Detector: tracks the last seen system/tools/model hash. Caller can
/// inspect `last_hash()` to decide whether the prompt "shape" changed
/// (in which case Anthropic's cache will miss and a fresh `cache_control`
/// block will be honoured by the server).
#[derive(Debug, Default, Clone)]
pub struct CacheBreakDetector {
    last_hash: Option<String>,
    stats: CacheMonitorStats,
}

impl CacheBreakDetector {
    /// New empty detector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the current request fingerprint and return `(changed, hash)`.
    /// `changed` is true if the hash differs from the previous observation.
    pub fn observe(&mut self, req: &ChatRequest) -> (bool, String) {
        let h = fingerprint(req);
        let changed = self.last_hash.as_deref() != Some(&h);
        self.last_hash = Some(h.clone());
        self.stats.observe_count += 1;
        if changed {
            self.stats.change_count += 1;
        }
        self.stats.last_hash = Some(h.clone());
        self.stats.last_changed = changed;
        (changed, h)
    }

    /// 只读累计统计(v1+ cache 监控)。
    pub fn stats(&self) -> &CacheMonitorStats {
        &self.stats
    }

    /// 重置统计计数(测试 / 新 session)。
    pub fn reset_stats(&mut self) {
        self.stats = CacheMonitorStats::default();
    }

    /// Most recent observed hash (or `None` if `observe` was never called).
    pub fn last_hash(&self) -> Option<&str> {
        self.last_hash.as_deref()
    }

    /// Reset the detector so the next observation will be reported as changed.
    pub fn reset(&mut self) {
        self.last_hash = None;
        self.reset_stats();
    }
}

/// Stable sha1 fingerprint of the request shape that affects cache hits.
/// Includes system blocks, tool specs, and model name.
fn fingerprint(req: &ChatRequest) -> String {
    let mut hasher = Sha1::new();
    // System blocks
    for b in &req.system.0 {
        hasher.update(b.text.as_bytes());
        hasher.update([0x00]);
        if b.ephemeral {
            hasher.update([0x01]);
        }
    }
    hasher.update([0xff]);
    // Tools (name + JSON params; description excluded — reflects reflect's
    // behaviour: tools description doesn't change cache hit).
    for t in &req.tools {
        let reflect_llm::ToolSpec::Function {
            name, parameters, ..
        } = t;
        hasher.update(name.as_bytes());
        hasher.update([0x00]);
        hasher.update(parameters.to_string().as_bytes());
        hasher.update([0x00]);
    }
    hasher.update([0xff]);
    hasher.update(req.model.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(40);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Find the message index of the prefix anchor breakpoint.
///
/// Uses Python-style negative indexing: `offset = -1` is the LAST
/// non-meta message, `offset = -2` is the 2nd-from-last, `offset = 0`
/// is the first. `offset = -2` is reflect's "2nd-from-last" default
/// (set `DEFAULT_PREFIX_ANCHOR_OFFSET` accordingly).
/// Returns `None` if there is no eligible non-meta message.
pub fn find_prefix_anchor(messages: &[ChatMessage], offset: i32) -> Option<usize> {
    let non_meta: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| !matches!(m, ChatMessage::System(_) | ChatMessage::Tool(_)))
        .map(|(i, _)| i)
        .collect();
    if non_meta.is_empty() {
        return None;
    }
    let idx = if offset < 0 {
        non_meta.len() as i32 + offset
    } else {
        offset
    };
    let clamped = idx.clamp(0, non_meta.len() as i32 - 1);
    non_meta.get(clamped as usize).copied()
}

/// Inject `cache_control` breakpoints into the request. Mutates `req` in place.
///
/// - If `caps.prompt_caching` is false → no-op.
/// - Always attaches `Ephemeral` + 可配 TTL 到 LAST `SystemBlock`(mandatory)。
/// - If `prefix_anchor_offset` resolves to a valid non-meta message index,
///   attaches `Ephemeral` + 可配 TTL 到该位置(Note: as of v0, `ChatMessage`
///   does not carry per-message `cache_control`; we push a `CacheBreak {
///   after_message_index, ttl }` entry into `req.cache_control` so the
///   provider implementation can emit the correct wire shape.)
/// - Optionally attaches `Ephemeral` + 可配 TTL 到 LAST tool spec via
///   `req.metadata["cache_break_tool"] = "true"`(provider implementations
///   check this flag)。
///
/// TTL 由 `system_cache_ttl()` 决定(读 `REFLECT_CACHE_TTL`,默认 `1h`)。
pub fn inject_cache_control(
    req: &mut ChatRequest,
    caps: &Capabilities,
) -> Result<(), CacheBreakError> {
    if !caps.prompt_caching {
        return Err(CacheBreakError::Unsupported);
    }

    let ttl = system_cache_ttl();

    // 1. Mandatory breakpoint at end of last SystemBlock.
    if let Some(last) = req.system.0.last_mut() {
        last.cache_control = Some(default_cc(ttl));
    }

    // 2. Prefix anchor breakpoint (via CacheBreak on message index).
    if let Some(anchor) = find_prefix_anchor(&req.messages, DEFAULT_PREFIX_ANCHOR_OFFSET) {
        req.cache_control.push(reflect_llm::CacheBreak {
            after_message_index: anchor,
            ttl,
        });
    }

    // 3. Last tool cache control hint (provider reads metadata flag).
    if !req.tools.is_empty() {
        req.metadata
            .insert("cache_break_tool".to_string(), "true".to_string());
    }
    Ok(())
}

/// Force-set `cache_control: Ephemeral` + 可配 TTL on a specific `SystemBlock`。
pub fn force_break_on_block(block: &mut SystemBlock) {
    block.cache_control = Some(default_cc(system_cache_ttl()));
}

/// 解析 `REFLECT_CACHE_TTL` 环境变量返回系统 prompt 缓存 TTL。
///
/// - `"5m"` → `CacheTtl::FiveMinutes`(对齐 Anthropic 5 分钟短缓存)。
/// - 未设置 / 其它值 → `CacheTtl::OneHour`(默认;长生命周期的 agent
///   system prompt 用 1h 命中率更高)。沿用 `REFLECT_AUTO_COMPACT_INPUT_TOKENS`
///   的 env 惯例。
///
/// 每次调用都重读 env,便于测试在同进程内翻转(沿用模块级 env-lock 模式)。
pub fn system_cache_ttl() -> CacheTtl {
    match std::env::var("REFLECT_CACHE_TTL")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("5m") => CacheTtl::FiveMinutes,
        _ => CacheTtl::OneHour,
    }
}

fn default_cc(ttl: CacheTtl) -> CacheControl {
    CacheControl {
        kind: CacheControlKind::Ephemeral,
        ttl: Some(ttl),
    }
}

/// Empty `SystemBlocks` value (re-exported for convenience).
pub fn empty_blocks() -> SystemBlocks {
    SystemBlocks::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::{AssistantContent, ContentBlock, ToolResult, ToolSpec, UserContent};
    use serde_json::json;

    fn chat_request() -> ChatRequest {
        ChatRequest {
            model: "claude-3-5-sonnet".into(),
            ..Default::default()
        }
    }

    fn make_user(s: &str) -> ChatMessage {
        ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text(s)],
        })
    }

    fn make_assistant(s: &str) -> ChatMessage {
        ChatMessage::Assistant(AssistantContent {
            text: Some(s.into()),
            tool_calls: vec![],
            thinking: None,
        })
    }

    fn make_tool(id: &str, s: &str) -> ChatMessage {
        ChatMessage::Tool(ToolResult {
            call_id: id.into(),
            content: s.into(),
            is_error: false,
        })
    }

    fn make_system(s: &str) -> ChatMessage {
        ChatMessage::System(s.into())
    }

    #[test]
    fn find_prefix_anchor_offset_minus_one_picks_last_non_meta() {
        // [User, Assistant, User] → non-meta indices = [0, 1, 2]; offset=-1 → index 2
        let msgs = vec![make_user("a"), make_assistant("b"), make_user("c")];
        assert_eq!(find_prefix_anchor(&msgs, -1), Some(2));
    }

    #[test]
    fn find_prefix_anchor_offset_minus_two_picks_second_to_last_non_meta() {
        // offset=-2 → 2nd from last non-meta = index 1
        let msgs = vec![make_user("a"), make_assistant("b"), make_user("c")];
        assert_eq!(find_prefix_anchor(&msgs, -2), Some(1));
    }

    #[test]
    fn find_prefix_anchor_offset_zero_picks_first_non_meta() {
        let msgs = vec![make_user("a"), make_assistant("b")];
        assert_eq!(find_prefix_anchor(&msgs, 0), Some(0));
    }

    #[test]
    fn find_prefix_anchor_skips_system_and_tool() {
        // [System, User, Assistant, Tool] → non-meta = [1, 2]
        let msgs = vec![
            make_system("s"),
            make_user("a"),
            make_assistant("b"),
            make_tool("c1", "ok"),
        ];
        // offset=-1 → 2nd non-meta = index 2
        assert_eq!(find_prefix_anchor(&msgs, -1), Some(2));
    }

    #[test]
    fn find_prefix_anchor_none_if_no_non_meta() {
        let msgs = vec![make_system("s"), make_tool("c1", "ok")];
        assert_eq!(find_prefix_anchor(&msgs, -1), None);
    }

    #[test]
    fn inject_skips_when_no_caching() {
        let mut req = chat_request();
        let caps = Capabilities::default();
        let err = inject_cache_control(&mut req, &caps).unwrap_err();
        assert!(matches!(err, CacheBreakError::Unsupported));
        // SystemBlock should be untouched.
        assert!(req.system.0.is_empty());
    }

    #[test]
    fn inject_sets_last_system_block_cache_control() {
        let mut req = chat_request();
        req.system = SystemBlocks(vec![
            SystemBlock {
                text: "first".into(),
                cache_control: None,
                ephemeral: false,
            },
            SystemBlock {
                text: "last".into(),
                cache_control: None,
                ephemeral: false,
            },
        ]);
        let caps = Capabilities {
            prompt_caching: true,
            ..Capabilities::default()
        };
        inject_cache_control(&mut req, &caps).unwrap();
        assert!(req.system.0[0].cache_control.is_none());
        let last_cc = req.system.0[1].cache_control.expect("last block has cc");
        assert!(matches!(last_cc.kind, CacheControlKind::Ephemeral));
    }

    #[test]
    fn inject_creates_empty_system_block_with_cc_if_none() {
        let mut req = chat_request();
        // No system blocks at all.
        let caps = Capabilities {
            prompt_caching: true,
            ..Capabilities::default()
        };
        inject_cache_control(&mut req, &caps).unwrap();
        // No block to attach to: stays empty (we don't fabricate one).
        assert!(req.system.0.is_empty());
    }

    #[test]
    fn inject_records_prefix_anchor_cache_break() {
        // TTL 由 `system_cache_ttl()`(读 env)决定,具体值在
        // `system_cache_ttl_reads_env_and_inject_honors_it` 单测覆盖;本测
        // 聚焦 prefix-anchor 索引(不绑定具体 TTL,避免 env 并行竞争)。
        let mut req = chat_request();
        req.system = SystemBlocks(vec![SystemBlock {
            text: "core".into(),
            cache_control: None,
            ephemeral: false,
        }]);
        req.messages = vec![make_user("a"), make_assistant("b"), make_user("c")];
        let caps = Capabilities {
            prompt_caching: true,
            ..Capabilities::default()
        };
        inject_cache_control(&mut req, &caps).unwrap();
        // non-meta = [0,1,2], offset=-1 → 2
        assert_eq!(req.cache_control.len(), 1);
        assert_eq!(req.cache_control[0].after_message_index, 2);
        // TTL 永远被注入(FiveMinutes 或 OneHour,取决于 env)。
        assert!(matches!(
            req.cache_control[0].ttl,
            CacheTtl::FiveMinutes | CacheTtl::OneHour
        ));
    }

    #[test]
    fn inject_sets_cache_break_tool_metadata_when_tools_present() {
        let mut req = chat_request();
        req.tools.push(ToolSpec::Function {
            name: "read".into(),
            description: "Read a file".into(),
            parameters: json!({"type": "object"}),
        });
        let caps = Capabilities {
            prompt_caching: true,
            ..Capabilities::default()
        };
        inject_cache_control(&mut req, &caps).unwrap();
        assert_eq!(
            req.metadata.get("cache_break_tool").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn detector_observes_no_change_when_unchanged() {
        let mut req = chat_request();
        req.system = SystemBlocks(vec![SystemBlock {
            text: "core".into(),
            cache_control: None,
            ephemeral: false,
        }]);
        let mut det = CacheBreakDetector::new();
        let (c1, h1) = det.observe(&req);
        assert!(c1);
        let (c2, h2) = det.observe(&req);
        assert!(!c2);
        assert_eq!(h1, h2);
    }

    #[test]
    fn detector_observes_change_when_system_text_differs() {
        let mut req = chat_request();
        req.system = SystemBlocks(vec![SystemBlock {
            text: "v1".into(),
            cache_control: None,
            ephemeral: false,
        }]);
        let mut det = CacheBreakDetector::new();
        let (c1, h1) = det.observe(&req);
        assert!(c1);
        // Change system.
        req.system.0[0].text = "v2".into();
        let (c2, h2) = det.observe(&req);
        assert!(c2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn detector_observes_change_when_model_differs() {
        let mut req = chat_request();
        req.model = "claude-3-5-sonnet".into();
        let mut det = CacheBreakDetector::new();
        let (c1, h1) = det.observe(&req);
        assert!(c1);
        req.model = "claude-3-5-haiku".into();
        let (c2, h2) = det.observe(&req);
        assert!(c2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn detector_resets() {
        let req = chat_request();
        let mut det = CacheBreakDetector::new();
        let _ = det.observe(&req);
        det.reset();
        let (c, _) = det.observe(&req);
        assert!(c, "reset should make next observation appear changed");
    }

    #[test]
    fn detector_stats_track_observe_and_change() {
        let mut req = chat_request();
        req.model = "claude-3-5-sonnet".into();
        let mut det = CacheBreakDetector::new();
        let _ = det.observe(&req);
        assert_eq!(det.stats().observe_count, 1);
        assert_eq!(det.stats().change_count, 1);
        let _ = det.observe(&req);
        assert_eq!(det.stats().observe_count, 2);
        assert_eq!(det.stats().change_count, 1);
        assert!(det.stats().hit_rate() > 0.0);
        req.model = "claude-3-5-haiku".into();
        let _ = det.observe(&req);
        assert_eq!(det.stats().change_count, 2);
    }

    #[test]
    fn force_break_on_block_sets_cc() {
        let mut b = SystemBlock {
            text: "x".into(),
            cache_control: None,
            ephemeral: false,
        };
        force_break_on_block(&mut b);
        assert!(b.cache_control.is_some());
    }

    // ── TTL 可配(REFLECT_CACHE_TTL) ──────────────────────────────────────
    //
    // 注:env 是进程级全局可变状态,并行测试会互相干扰。本模块把所有
    // env-flipping 断言收进同一个 test fn,避免跨 test 竞争。

    /// TTL 可配性 + inject 联动:单 test 内顺序翻转 `REFLECT_CACHE_TTL`,
    /// 验证 `system_cache_ttl()` 与 `inject_cache_control` 都尊重 env。
    /// 默认(清除)→ OneHour;`"5m"` → FiveMinutes;未知值 → OneHour。
    /// `5m` 时 inject 注入的 system 块 + prefix-anchor 断点都用 FiveMinutes。
    #[test]
    fn system_cache_ttl_reads_env_and_inject_honors_it() {
        // 1. 默认(清除 env)→ OneHour。
        unsafe { std::env::remove_var("REFLECT_CACHE_TTL") };
        assert!(matches!(system_cache_ttl(), CacheTtl::OneHour));

        // 2. `"5m"` → FiveMinutes,且 inject 注入的断点也用 5m。
        unsafe { std::env::set_var("REFLECT_CACHE_TTL", "5m") };
        assert!(matches!(system_cache_ttl(), CacheTtl::FiveMinutes));
        {
            let mut req = chat_request();
            req.system = SystemBlocks(vec![SystemBlock {
                text: "core".into(),
                cache_control: None,
                ephemeral: false,
            }]);
            req.messages = vec![make_user("a"), make_assistant("b"), make_user("c")];
            let caps = Capabilities {
                prompt_caching: true,
                ..Capabilities::default()
            };
            inject_cache_control(&mut req, &caps).unwrap();
            let cc = req.system.0[0]
                .cache_control
                .as_ref()
                .expect("system block has cc");
            assert!(matches!(cc.ttl, Some(CacheTtl::FiveMinutes)));
            assert_eq!(req.cache_control.len(), 1);
            assert!(matches!(req.cache_control[0].ttl, CacheTtl::FiveMinutes));
        }

        // 3. 未知值 → 回退 OneHour。
        unsafe { std::env::set_var("REFLECT_CACHE_TTL", "garbage") };
        assert!(matches!(system_cache_ttl(), CacheTtl::OneHour));

        // 4. 清理:避免污染同进程其它测试。
        unsafe { std::env::remove_var("REFLECT_CACHE_TTL") };
    }
}
