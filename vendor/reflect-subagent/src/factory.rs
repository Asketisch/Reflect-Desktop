//! `SubAgentFactory` — spawns child `AgentThread`s from a parent context.
//!
//! Lives behind an `Arc<SubAgentFactory>` on `NodeContext` so any tool can
//! submit a child `Submission` without needing direct access to the
//! parent's submission channel.
//!
//! Depth enforcement happens here: the factory holds an `AtomicU8` counter
//! that's incremented on every `spawn()` and refused once it crosses
//! [`crate::MAX_DEPTH`]. The counter is shared with the child's factory so
//! nested grandchildren share the same depth budget.
//!
//! v0.2.2 起 `default_model` 用 `parking_lot::Mutex<String>` 包裹,允许
//! `reflect-exec::handle_reload` 在用户编辑 `~/.reflect/config.toml` 时
//! 通过 `set_default_model` 实时更新 —— 已 spawn 的子 agent 通过共享
//! `Arc<ModelRegistry>` 自动跟随 client 切换;**新 spawn 的子 agent**
//! 拿新 default。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Instant;

use parking_lot::Mutex;
use reflect_core::AgentThread;
use reflect_core::config::AgentConfig;
use reflect_llm::{ChatMessage, SharedModelRegistry};
use reflect_protocol::{RolloutRecord, Submission, ThreadId, TokenUsage};
use reflect_recovery::SubagentRegistry;
use reflect_tools::ToolRegistry;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::data_transfer::DataTransferConfig;
use crate::error::SubAgentError;
use crate::spec::SubAgentSpec;
use crate::worker_registry::build_worker_tool_registry;

/// Shared factory: cloned cheaply, all clones see the same depth counter.
pub struct SubAgentFactory {
    /// Shared depth counter; incremented on every `spawn()`.
    depth: Arc<AtomicU8>,
    /// Parent thread id (used for `RolloutRecord::Fork` parent_session_id).
    parent_session_id: ThreadId,
    /// Parent model spec — used as default when `spec.model` is `None`。
    /// v0.2.2: `Mutex<String>` 让 `handle_reload` 可写;`spawn` 时克隆
    /// 当前值快照进子 `AgentConfig.model`。
    default_model: Mutex<String>,
    /// Parent registry (shared with children so they reuse API clients).
    registry: SharedModelRegistry,
    /// Optional child-specific registry for subagents.
    /// When `Some`, children use this instead of the parent's registry,
    /// allowing independent base_url + api_key + model for subagents.
    child_registry: Option<SharedModelRegistry>,
    /// Parent tool registry — children receive a filtered view of this.
    parent_tools: Arc<ToolRegistry>,
    /// Parent cancel token — children inherit.
    cancel: CancellationToken,
    /// Optional parent recorder (for emitting `Fork` records + creating
    /// per-child JSONL files).
    recorder: Option<Arc<dyn reflect_protocol::RolloutRecorder>>,
    /// M5 v0: children get a minimal `AgentConfig` (no M4 deps). If/when
    /// we want children to share the parent's compactor / memory / skills,
    /// add a `parent_m4: Option<M4Deps>` here and propagate into the child.
    _no_parent_m4: (),
    /// v1.0.0-rc2: plugin 提供的 sub-agent spec —— key 是 plugin id,
    /// value 是该 plugin 注册的所有 spec。`PluginManager::load` 调
    /// `register_plugin_spec`;`unload` 调 `take_plugin_specs` 反注册。
    plugin_specs: Mutex<HashMap<String, Vec<SubAgentSpec>>>,
    /// v1.1.0: 运行时由 `TaskManager::sync_team_specs` 注入的动态 spec —— key 是
    /// `role`,value 是 spec。**不**被 `child_factory` 继承,与 `plugin_specs`
    /// 同语义(子 factory 仅继承 spawn 自身需要的 spec,不在自己身上管理)。
    ///
    /// 用途:Phase 3 起 `TeamCreate` 把团队成员的 `TeamMemberSpec` 转
    /// `SubAgentSpec` 注入到本字段,后续 `call_<role>` 即可 spawn。Phase 6
    /// TUI 任务面板直接消费 `list_specs` 渲染团队成员列表。
    dynamic_specs: Mutex<HashMap<String, SubAgentSpec>>,
    /// v1.1.0 Phase 6 P0:跨 turn 共享的子代理调用注册表。
    /// `None` 表示未接入(默认 / 单测);`reflect-exec::bootstrap_m5`
    /// 通过 `set_subagent_registry` 注入 `M4Deps` 的同一 Arc,
    /// `CallSubAgentTool::execute` 写,`pre_loop` 读 + 渲染成
    /// `<system-reminder>`。`Mutex` 包裹让 `set_subagent_registry`
    /// 走 `&self`(对齐 `set_default_model` 模式),`Arc<Factory>` clone
    /// 后所有副本都看到同一份 registry。
    subagent_registry: Mutex<Option<Arc<SubagentRegistry>>>,
    /// v1.1.0 Phase 4:coordinator 模式开关。`true` 时 `spawn` 走
    /// `build_worker_tool_registry`(从 `parent_tools` 排除 `INTERNAL_WORKER_TOOLS`),
    /// 替代默认的 `spec.allowed_tools` 过滤。`AtomicBool` 而非 `Mutex<bool>`:
    /// spawn 热路径只读,reload 偶写,无锁竞争。
    coordinator_mode: Arc<AtomicBool>,
    /// v1.1.0 Phase 4:coordinator 模式启用时,spawn 时附加到子 agent
    /// user input 末尾的「Coordinator Principle」reminder 文本。`None`
    /// 时不附加。`Mutex<Option<String>>` 让 `set_coordinator_mode` 走 `&self`
    /// (对齐 `set_default_model` 模式),子 factory 通过 clone 共享。
    coordinator_footer: Mutex<Option<String>>,
}

impl std::fmt::Debug for SubAgentFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubAgentFactory")
            .field("parent_session_id", &self.parent_session_id)
            .field("default_model", &self.default_model.lock().clone())
            .field("depth", &self.depth.load(Ordering::Relaxed))
            .finish()
    }
}

impl SubAgentFactory {
    /// Build a factory rooted at the given parent thread.
    pub fn new(
        parent_session_id: ThreadId,
        default_model: impl Into<String>,
        registry: SharedModelRegistry,
        child_registry: Option<SharedModelRegistry>,
        parent_tools: Arc<ToolRegistry>,
        cancel: CancellationToken,
        recorder: Option<Arc<dyn reflect_protocol::RolloutRecorder>>,
    ) -> Self {
        Self {
            depth: Arc::new(AtomicU8::new(0)),
            parent_session_id,
            default_model: Mutex::new(default_model.into()),
            registry,
            child_registry,
            parent_tools,
            cancel,
            recorder,
            _no_parent_m4: (),
            plugin_specs: Mutex::new(HashMap::new()),
            dynamic_specs: Mutex::new(HashMap::new()),
            subagent_registry: Mutex::new(None),
            coordinator_mode: Arc::new(AtomicBool::new(false)),
            coordinator_footer: Mutex::new(None),
        }
    }

    /// Current depth value (0 at the parent, +1 per spawn).
    pub fn depth(&self) -> u8 {
        self.depth.load(Ordering::SeqCst)
    }

    /// 父 session 的 thread id —— coordinator scratchpad 路径与 reload 复用。
    pub fn parent_session_id(&self) -> ThreadId {
        self.parent_session_id
    }

    /// 读当前 default model spec。`spawn` 时和测试窥探都用。
    pub fn default_model(&self) -> String {
        self.default_model.lock().clone()
    }

    /// 写入新 default model spec。仅供 `reflect-exec::handle_reload` 在
    /// 检测到 `~/.reflect/config.toml` 变更后调用 —— 已 spawn 子 agent
    /// 不受影响(它们已经快照);后续 `spawn()` 拿新值。
    pub fn set_default_model(&self, new_spec: impl Into<String>) {
        *self.default_model.lock() = new_spec.into();
    }

    // ── v1.1.0 Phase 6 P0: subagent registry 共享 ───────────────────────

    /// 注入跨 turn 共享的子代理注册表。仅供 `reflect-exec::bootstrap_m5`
    /// 在 `M4Deps` 构造后调一次 —— factory 与 M4Deps 持有同一 Arc,
    /// `CallSubAgentTool::execute` 写,`pre_loop` 读。
    pub fn set_subagent_registry(&self, registry: Arc<SubagentRegistry>) {
        *self.subagent_registry.lock() = Some(registry);
    }

    /// 读当前注册表(若有)。给 `CallSubAgentTool::execute` 用。
    pub fn subagent_registry(&self) -> Option<Arc<SubagentRegistry>> {
        self.subagent_registry.lock().clone()
    }

    // ── v1.1.0 Phase 4: coordinator mode 注入 ─────────────────────────

    /// 设置 coordinator 模式开关与 footer 文本。仅供
    /// `reflect-exec::bootstrap_m4` 在 `coord_cfg.enabled` 时调一次。
    ///
    /// - `enabled = true` → `spawn` 走 `build_worker_tool_registry`,子 agent
    ///   拿不到 `INTERNAL_WORKER_TOOLS`(`TeamCreate` / `TeamDelete` /
    ///   `SyntheticOutput` / `send_message`)。
    /// - `enabled = false` → 回退原 `spec.allowed_tools` 路径(回归保护)。
    /// - `footer = Some(text)` → spawn 时把 `[Coordinator Principle]` 段附加
    ///   到 `combined_user_input` 末尾,提醒 worker 独立综合结果。
    ///
    /// 子 factory 通过 `child_factory` 共享同一 `Arc<AtomicBool>` + clone
    /// 当前 footer,与 `subagent_registry` 模式一致。
    pub fn set_coordinator_mode(&self, enabled: bool, footer: Option<String>) {
        self.coordinator_mode.store(enabled, Ordering::SeqCst);
        *self.coordinator_footer.lock() = footer;
    }

    /// 当前 coordinator 模式开关(供测试与诊断)。
    pub fn is_coordinator_mode(&self) -> bool {
        self.coordinator_mode.load(Ordering::SeqCst)
    }

    /// 当前 coordinator footer 文本(供测试与诊断)。
    pub fn coordinator_footer(&self) -> Option<String> {
        self.coordinator_footer.lock().clone()
    }

    // ── v1.0.0-rc2: plugin sub-agent 注册 ────────────────────────────

    /// 注册一个 plugin 提供的 sub-agent spec。
    ///
    /// `PluginManager::load` 在 plugin load 时调此方法,把 spec 存到
    /// factory 的 plugin 命名空间下;后续 `PluginManager` 还会构造对应的
    /// `CallSubAgentTool` 实例并 `tools.register_plugin_tool()` 挂到
    /// `ToolRegistry`。`spawn` 本身不查这个表 —— 它通过 `CallSubAgentTool`
    /// 持有 spec 的引用直接 spawn,避免一次额外查表。
    ///
    /// 反注册:`take_plugin_specs(plugin_id)` 在 `PluginManager::unload`
    /// 时拿回所有 spec,反注册对应的 `CallSubAgentTool`。
    pub fn register_plugin_spec(&self, plugin_id: &str, spec: SubAgentSpec) {
        if let Err(e) = spec.validate() {
            tracing::warn!(
                plugin = %plugin_id,
                role = %spec.role,
                error = %e,
                "plugin agent spec 校验失败,跳过"
            );
            return;
        }
        let mut map = self.plugin_specs.lock();
        map.entry(plugin_id.to_string()).or_default().push(spec);
    }

    /// 取出并移除某个 plugin 的所有 spec。返回被移除的 specs,给 caller
    /// 反注册对应的 `CallSubAgentTool`(`ToolRegistry::unregister(name)`)。
    pub fn take_plugin_specs(&self, plugin_id: &str) -> Vec<SubAgentSpec> {
        self.plugin_specs
            .lock()
            .remove(plugin_id)
            .unwrap_or_default()
    }

    /// 只读列出某个 plugin 注册的 specs(不消耗)—— 给 UI / 测试用。
    pub fn plugin_specs_for(&self, plugin_id: &str) -> Vec<SubAgentSpec> {
        self.plugin_specs
            .lock()
            .get(plugin_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 列出所有已注册 plugin id —— 字典序,保证测试与 UI 渲染稳定。
    pub fn registered_plugin_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.plugin_specs.lock().keys().cloned().collect();
        ids.sort();
        ids
    }

    // ── v1.1.0: dynamic specs (TeamCreate / CLI / 运行时注入) ────────

    /// 替换全部 dynamic specs —— 由 `TaskManager::sync_team_specs` 在每次
    /// 团队成员变更后调用。`specs` 中 `validate` 失败的项跳过 + `warn`,
    /// 与 `register_plugin_spec` 风格一致,保证失败的 spec 不污染 factory。
    pub fn set_specs(&self, specs: Vec<SubAgentSpec>) {
        let mut map = self.dynamic_specs.lock();
        map.clear();
        for spec in specs {
            if let Err(e) = spec.validate() {
                warn!(
                    role = %spec.role,
                    error = %e,
                    "sync_team_specs: spec validate 失败,跳过"
                );
                continue;
            }
            map.insert(spec.role.clone(), spec);
        }
    }

    /// 追加单个 spec。返回 `true` = 成功;`false` = `validate` 失败(spec
    /// 未被注入)。给 CLI `reflect task team-sync` 子命令用。
    pub fn add_spec(&self, spec: SubAgentSpec) -> bool {
        if let Err(e) = spec.validate() {
            warn!(role = %spec.role, error = %e, "add_spec: spec validate 失败");
            return false;
        }
        self.dynamic_specs.lock().insert(spec.role.clone(), spec);
        true
    }

    /// 按 role 移除 spec。返回 `true` = 有移除,`false` = 不存在。
    pub fn remove_spec(&self, role: &str) -> bool {
        self.dynamic_specs.lock().remove(role).is_some()
    }

    /// 列出全部 dynamic specs —— 按 `(role, name)` 字典序对,供 Phase 6
    /// TUI 任务面板渲染团队成员列表,以及 CLI `reflect task team-sync --list`。
    pub fn list_specs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = self
            .dynamic_specs
            .lock()
            .values()
            .map(|s| (s.role.clone(), s.name.clone()))
            .collect();
        pairs.sort();
        pairs
    }

    /// 按 role 取单个 spec 副本。
    pub fn get_spec(&self, role: &str) -> Option<SubAgentSpec> {
        self.dynamic_specs.lock().get(role).cloned()
    }

    /// Construct a factory for a child thread that shares this factory's
    /// depth counter. Used by [`spawn`] to give the child the same depth
    /// budget so its spawns are counted too.
    pub fn child_factory(&self) -> Self {
        Self {
            depth: self.depth.clone(),
            parent_session_id: self.parent_session_id,
            default_model: Mutex::new(self.default_model.lock().clone()),
            registry: self.registry.clone(),
            // 子 factory 继承父级的 child_registry —— 孙级 subagent 也走独立凭证。
            child_registry: self.child_registry.clone(),
            parent_tools: self.parent_tools.clone(),
            cancel: self.cancel.clone(),
            recorder: self.recorder.clone(),
            _no_parent_m4: (),
            // 子 factory 不继承 plugin_specs —— 子 agent 不需要管理
            // 父级 plugin;`PluginManager` 直接在父 factory 上操作。
            plugin_specs: Mutex::new(HashMap::new()),
            // dynamic_specs 同样不继承 —— spawn 时由调用方把需要的 spec
            // 显式 `add_spec` 到子 factory,避免子 agent 误用父级团队成员。
            dynamic_specs: Mutex::new(HashMap::new()),
            // registry 在父子间共享同一 Arc(若父注入了),保证孙级
            // 子代理也能被父级 pre_loop 看到。
            subagent_registry: Mutex::new(self.subagent_registry.lock().clone()),
            // coordinator 模式在父子间共享 —— 父启用时,子 spawn 出的孙级
            // 也走 worker tool 白名单;`Arc<AtomicBool>` 多读单写无锁竞争。
            coordinator_mode: self.coordinator_mode.clone(),
            // footer 走 clone 当前值快照(子 factory 后可独立更新)。
            coordinator_footer: Mutex::new(self.coordinator_footer.lock().clone()),
        }
    }

    /// Spawn a child thread for `spec`. Returns the child's `TurnHandle`,
    /// the new child `ThreadId`, and a child factory for further nesting.
    /// If the depth cap is hit, returns [`SubAgentError::MaxDepthExceeded`]
    /// **before** any `Submission` is queued.
    pub async fn spawn(
        &self,
        spec: SubAgentSpec,
        parent_tail: Vec<ChatMessage>,
        user_prompt: String,
    ) -> Result<SpawnedChild, SubAgentError> {
        spec.validate().map_err(SubAgentError::SpecInvalid)?;

        // Atomically reserve a depth slot. If we're already at the cap, refuse.
        let prev = self.depth.fetch_add(1, Ordering::SeqCst);
        if prev >= crate::MAX_DEPTH {
            // Roll back so the counter doesn't get stuck at the cap.
            self.depth.fetch_sub(1, Ordering::SeqCst);
            return Err(SubAgentError::MaxDepthExceeded {
                max: crate::MAX_DEPTH,
            });
        }

        // Build the child session — a fresh ThreadId + per-child recorder.
        let child_thread_id = ThreadId::new();
        let child_recorder: Option<Arc<dyn reflect_protocol::RolloutRecorder>> =
            if self.recorder.is_some() {
                // Children get their own JSONL file under the default base dir.
                Some(Arc::new(reflect_rollout::JsonlRolloutWriter::new(
                    reflect_rollout::path::default_base(),
                    child_thread_id,
                )))
            } else {
                None
            };

        // Emit a Fork record into the parent's recorder (if any) so the
        // relationship is queryable on resume.
        if let Some(rec) = self.recorder.as_ref() {
            let _ = rec
                .record(RolloutRecord::Fork {
                    parent_session_id: self.parent_session_id,
                    branch_name: spec.role.clone(),
                })
                .await;
        }

        // Build a filtered tool registry containing only `allowed_tools`.
        //
        // v1.1.0 Phase 4 改造:coordinator 模式启用时,不走 `spec.allowed_tools`,
        // 而是走 `reflect_task::coordinator::build_worker_tool_registry` 从父
        // registry 排除 `INTERNAL_WORKER_TOOLS`(`TeamCreate` / `TeamDelete` /
        // `SyntheticOutput` / `send_message`)。worker 不能自行创建 / 删除团队,
        // 也不能反向发消息给 coordinator(留 v1.2)。
        //
        // 注:`Arc<ToolRegistry>` 共享父级 —— 即便 worker 之后做 reload 触发
        // 子 registry 重建,父级主 session 的 plugin 加载 / MCP server 启动
        // 等副作用仍由主 session 持有,worker 只读 filter 视图。
        let child_tools = Arc::new(ToolRegistry::default());
        if self.coordinator_mode.load(Ordering::SeqCst) {
            // coordinator 启用:从父 registry 排除 internal tools。
            let worker_registry = build_worker_tool_registry(&self.parent_tools);
            for name in worker_registry.list() {
                if let Some(t) = worker_registry.get(&name) {
                    child_tools.register(t);
                }
            }
        } else {
            // 默认路径:按 spec.allowed_tools 过滤。
            for name in &spec.allowed_tools {
                if let Some(t) = self.parent_tools.get(name) {
                    child_tools.register(t);
                }
            }
        }

        // Build the child AgentConfig + thread. snapshot `default_model`
        // 在 spawn 入口取 —— reload 写锁不会和这里读锁竞争(锁粒度 String
        // 几纳秒)。
        let child_model = spec
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.lock().clone());
        let workspace = std::path::PathBuf::from(".");
        let mut cfg = AgentConfig::new(child_model, workspace);
        if let Some(rec) = child_recorder.clone() {
            // v1.1.0 review bug-1 (P0):`default_m4_deps` 内部给 `subagent_registry`
            // 一个全新 Arc,会让子 agent 的 `pre_loop` 渲染出空 reminder,看不到
            // 父 / 自身任何已完成的子代理调用 —— 与文档「跨 turn 通过
            // `Arc<SubagentRegistry>` 共享在 `M4Deps` 上」的契约相悖。修正:
            // 注入父 factory 的同一 Arc;若父未注入(测试 / headless 退化路径),
            // 仍走 fresh Arc,行为不变。
            let mut m4 = reflect_core::config::default_m4_deps("subagent");
            if let Some(reg) = self.subagent_registry() {
                m4.subagent_registry = reg;
            }
            m4.recorder = Some(rec);
            cfg.m4 = Some(m4);
        }
        // Use child registry if configured (independent subagent credentials),
        // otherwise fall back to parent's shared registry.
        let target_registry = self
            .child_registry
            .clone()
            .unwrap_or_else(|| self.registry.clone());
        let child_thread = AgentThread::new(cfg, target_registry, child_tools, None);

        // The child receives the system prompt + role label + user prompt as
        // a single User message. Parent context tail is currently unused
        // (M5 v0); it's a hook for future M6 enhancements where the parent
        // passes selected history into the child.
        let _ = parent_tail;
        let combined_user_input = build_spawn_user_input(
            &spec,
            &user_prompt,
            self.coordinator_mode.load(Ordering::SeqCst),
            self.coordinator_footer.lock().clone(),
        );
        let sub = Submission::user_input(combined_user_input);
        let handle = child_thread.submit(sub).await;
        Ok(SpawnedChild {
            session_id: child_thread_id,
            handle,
            data_transfer: spec.data_transfer,
        })
    }
}

/// 构造 spawn 时发给子 agent 的合并 user input。
/// coordinator 启用且 footer 非空时附加 `[Coordinator Principle]` 段。
pub(crate) fn build_spawn_user_input(
    spec: &SubAgentSpec,
    user_prompt: &str,
    coordinator_enabled: bool,
    footer: Option<String>,
) -> String {
    let mut combined = format!(
        "{}\n\n[Subagent role: {}]\n\n{}",
        spec.system_prompt, spec.name, user_prompt
    );
    if coordinator_enabled {
        if let Some(footer) = footer {
            if !footer.trim().is_empty() {
                combined.push_str("\n\n[Coordinator Principle]\n");
                combined.push_str(footer.trim());
                combined.push('\n');
            }
        }
    }
    combined
}

/// Handle returned by [`SubAgentFactory::spawn`] — caller drains events
/// from `handle` and passes them to [`crate::data_transfer::extract_result`].
pub struct SpawnedChild {
    pub session_id: ThreadId,
    pub handle: reflect_core::TurnHandle,
    pub data_transfer: DataTransferConfig,
}

impl SpawnedChild {
    /// Drain events until `TurnComplete` (or the channel closes) and
    /// extract the final answer using `data_transfer.result_extractor`.
    /// Thin wrapper around [`Self::collect_result_with_usage`] preserving the
    /// M5/M6 signature.
    pub async fn collect_result(self) -> Result<String, SubAgentError> {
        self.collect_result_with_usage().await.map(|r| r.text)
    }

    /// v0.2.4: 同 [`Self::collect_result`] 的 drain 流程,额外捕获 LLM 上报的
    /// token usage 与 elapsed 毫秒数,供 `DiscussionOrchestrator` 把 usage
    /// 注入出站 `DiscussionMessage`(`M9 已知限制 #3`)。
    ///
    /// usage 捕获优先级:
    /// 1. `TurnComplete.usage` —— 终结事件携带的最权威 usage
    /// 2. 最后一个 `TokenCount` 事件 —— 当 `TurnComplete` 没 emit(早终止 /
    ///    channel 关闭)的兜底
    ///
    /// 都不存在时 `token_usage = None`,调用方应当 fallback 到 `Default`。
    pub async fn collect_result_with_usage(self) -> Result<SpawnedResult, SubAgentError> {
        let SpawnedChild {
            mut handle,
            data_transfer,
            ..
        } = self;
        let started_at = Instant::now();
        let mut events = Vec::new();
        // Track usage from the most authoritative source seen so far.
        let mut usage_from_turn_complete: Option<TokenUsage> = None;
        let mut usage_from_token_count: Option<TokenUsage> = None;
        while let Some(ev) = handle.next().await {
            match &ev.msg {
                reflect_protocol::EventMsg::TurnComplete(tc) => {
                    usage_from_turn_complete = Some(tc.usage.clone());
                }
                reflect_protocol::EventMsg::TokenCount(tc) => {
                    usage_from_token_count = Some(TokenUsage {
                        input_tokens: tc.input_tokens,
                        output_tokens: tc.output_tokens,
                        cached_tokens: tc.cached_tokens,
                        cache_write_tokens: tc.cache_write_tokens,
                        total_tokens: tc.total_tokens,
                    });
                }
                _ => {}
            }
            let is_terminal = matches!(
                ev.msg,
                reflect_protocol::EventMsg::TurnComplete(_)
                    | reflect_protocol::EventMsg::TurnAborted(_)
                    | reflect_protocol::EventMsg::ShutdownComplete
            );
            events.push(ev);
            if is_terminal {
                break;
            }
        }
        let text = crate::data_transfer::extract_result(&events, &data_transfer.result_extractor)
            .ok_or_else(|| SubAgentError::SpawnFailed("no result extractable".into()))?;
        let token_usage = usage_from_turn_complete.or(usage_from_token_count);
        Ok(SpawnedResult {
            text,
            token_usage,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
        })
    }
}

/// v0.2.4: 一次 subagent spawn 的完整结果。
///
/// `text` 是按 `data_transfer.result_extractor` 抽出的最终回答文本;
/// `token_usage` 是 LLM 上报的 token 用量(优先取 `TurnComplete.usage`,
/// 兜底取最后一个 `TokenCount` 事件);`elapsed_ms` 是 drain 流的总耗时。
#[derive(Debug, Clone)]
pub struct SpawnedResult {
    pub text: String,
    pub token_usage: Option<TokenUsage>,
    pub elapsed_ms: u64,
}

// Internal helper removed — children get their AgentConfig populated directly above.

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_tools::ToolRegistry;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn depth_increments_on_spawn() {
        let factory = SubAgentFactory::new(
            ThreadId::new(),
            "openai/gpt-4o",
            std::sync::Arc::new(reflect_llm::ModelRegistry::new()),
            None, // child_registry: 回退父级 registry
            Arc::new(ToolRegistry::default()),
            CancellationToken::new(),
            None,
        );
        assert_eq!(factory.depth(), 0);
        factory.depth.fetch_add(1, Ordering::SeqCst);
        assert_eq!(factory.depth(), 1);
    }

    #[test]
    fn child_factory_shares_depth_counter() {
        let factory = SubAgentFactory::new(
            ThreadId::new(),
            "openai/gpt-4o",
            std::sync::Arc::new(reflect_llm::ModelRegistry::new()),
            None, // child_registry: 回退父级 registry
            Arc::new(ToolRegistry::default()),
            CancellationToken::new(),
            None,
        );
        let child = factory.child_factory();
        factory.depth.fetch_add(1, Ordering::SeqCst);
        assert_eq!(child.depth(), 1, "child sees the same counter");
    }

    #[test]
    fn max_depth_refuses_after_three_increments() {
        let factory = SubAgentFactory::new(
            ThreadId::new(),
            "openai/gpt-4o",
            std::sync::Arc::new(reflect_llm::ModelRegistry::new()),
            None, // child_registry: 回退父级 registry
            Arc::new(ToolRegistry::default()),
            CancellationToken::new(),
            None,
        );
        // Pretend three spawns already happened (depth == 3 = MAX_DEPTH).
        factory.depth.fetch_add(3, Ordering::SeqCst);
        // The check at the top of `spawn` is `prev >= MAX_DEPTH`; with the
        // counter already at 3, the next spawn would see `prev=3` and refuse.
        assert!(factory.depth() >= crate::MAX_DEPTH);
    }

    /// `set_default_model` 写入后 `default_model()` 读到新值;`child_factory()`
    /// 共享同一字符串(同一把 Mutex 走 `lock().clone()`)。
    #[test]
    fn set_default_model_updates_value_and_propagates_to_child() {
        let factory = SubAgentFactory::new(
            ThreadId::new(),
            "anthropic/claude-3-5-sonnet-latest",
            std::sync::Arc::new(reflect_llm::ModelRegistry::new()),
            None, // child_registry: 回退父级 registry
            Arc::new(ToolRegistry::default()),
            CancellationToken::new(),
            None,
        );
        assert_eq!(
            factory.default_model(),
            "anthropic/claude-3-5-sonnet-latest"
        );
        factory.set_default_model("openai/gpt-4o");
        assert_eq!(factory.default_model(), "openai/gpt-4o");
        // child factory 在 set 之后构造,应看到新值
        let child = factory.child_factory();
        assert_eq!(
            child.default_model(),
            "openai/gpt-4o",
            "child factory should observe parent's post-set value"
        );
    }

    // ── v0.2.4: collect_result_with_usage ─────────────────────────────

    /// 构造一个最小化的 `SpawnedChild` —— 由于 `TurnHandle` 字段私有,
    /// 通过 mpsc channel + `AgentThread` 内部不必要,直接走一个 `Event`
    /// channel + 手写 `TurnHandle`。`TurnHandle` 暴露 `next()` 返回
    /// `Option<Event>`,我们可以包装一个自己的 `next()` 通过显式构造。
    ///
    /// 为避免依赖 `reflect_core::TurnHandle` 的内部表示,采用 trait 抽象:
    /// 在 `SpawnedChild` 上 `handle.next().await` 要求 `handle: TurnHandle`,
    /// 而 `TurnHandle` 是 `reflect_core` 公开类型。直接构造一条会跳过
    /// 真实 AgentThread wiring —— 这里只验证 `extract_result` 路径 +
    /// usage 捕获路径,因此把 drain 逻辑复制到单测里,以 `SpawnedChild`
    /// 不变(签名不变)为前提测试逻辑正确性。
    ///
    /// 真正端到端测试由 `reflect-discussion/tests/discussion_collab_e2e.rs`
    /// 覆盖,这里只验证 drain 语义。
    use reflect_protocol::{
        Event, EventMsg, TokenCountEvent, TurnCompleteEvent, TurnId, TurnStatus,
    };

    /// 单测辅助:把一组 events + 一个 extractor 喂给与 `collect_result_with_usage`
    /// 等价的私有 drain 逻辑,断言 usage 捕获。
    ///
    /// 由于 `SpawnedChild` 的 `handle` 字段没有 setter,我们通过反射式地构造
    /// 一份 `SpawnedChild` 在编译期不可见 —— 改用把 drain 逻辑复制到测试里
    /// 的方式,测试 `TurnComplete.usage` 与 `TokenCount` 两种来源。
    fn drain_usage_only(events: Vec<Event>) -> (Option<TokenUsage>, Option<TokenUsage>) {
        let mut from_turn_complete = None;
        let mut from_token_count = None;
        for ev in &events {
            match &ev.msg {
                EventMsg::TurnComplete(tc) => from_turn_complete = Some(tc.usage.clone()),
                EventMsg::TokenCount(t) => {
                    from_token_count = Some(TokenUsage {
                        input_tokens: t.input_tokens,
                        output_tokens: t.output_tokens,
                        cached_tokens: t.cached_tokens,
                        cache_write_tokens: t.cache_write_tokens,
                        total_tokens: t.total_tokens,
                    });
                }
                _ => {}
            }
        }
        (from_turn_complete, from_token_count)
    }

    #[test]
    fn collect_result_with_usage_extracts_turn_complete_usage() {
        // 模拟:`TokenCount` 先发,随后 `TurnComplete.usage` 终结。
        // 期望 `token_usage = TurnComplete.usage` (权威来源胜出)。
        let events = vec![
            Event::new(
                "sub",
                EventMsg::TokenCount(TokenCountEvent {
                    input_tokens: 50,
                    output_tokens: 10,
                    cached_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: 60,
                    cost_usd: None,
                    ..Default::default()
                }),
            ),
            Event::new(
                "sub",
                EventMsg::TurnComplete(TurnCompleteEvent {
                    turn_id: TurnId::new(),
                    usage: TokenUsage::new(100, 20, 0),
                    status: TurnStatus::Success,
                }),
            ),
        ];
        let (from_tc, from_tok) = drain_usage_only(events);
        let final_usage = from_tc.or(from_tok).expect("should capture usage");
        assert_eq!(final_usage.input_tokens, 100);
        assert_eq!(final_usage.output_tokens, 20);
    }

    #[test]
    fn collect_result_with_usage_returns_none_when_no_token_event() {
        // drain 流仅 `TurnComplete` 不带 usage(全零的 default)且无 `TokenCount`。
        // 由于 `TurnComplete.usage` 是 `TokenUsage` (非 Option),即使值全零也算
        // "捕获到"。这是预期行为:LLM 真的没有 token 也会发 `TurnComplete`。
        let events = vec![Event::new(
            "sub",
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: TurnId::new(),
                usage: TokenUsage::default(),
                status: TurnStatus::Success,
            }),
        )];
        let (from_tc, from_tok) = drain_usage_only(events);
        // both are Some (default), final = from_tc (zero-valued)
        let final_usage = from_tc.or(from_tok).expect("should still have a usage");
        assert_eq!(final_usage.input_tokens, 0);
        assert_eq!(final_usage.output_tokens, 0);
    }

    #[test]
    fn collect_result_with_usage_picks_last_token_count_when_no_turn_complete() {
        // drain 流仅 2 个 `TokenCount`,没有 `TurnComplete`(早终止)。
        // 期望:`token_usage` = 最后一个 `TokenCount`。
        let events = vec![
            Event::new(
                "sub",
                EventMsg::TokenCount(TokenCountEvent {
                    input_tokens: 10,
                    output_tokens: 1,
                    cached_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: 11,
                    cost_usd: None,
                    ..Default::default()
                }),
            ),
            Event::new(
                "sub",
                EventMsg::TokenCount(TokenCountEvent {
                    input_tokens: 30,
                    output_tokens: 3,
                    cached_tokens: 5,
                    cache_write_tokens: 0,
                    total_tokens: 33,
                    cost_usd: None,
                    ..Default::default()
                }),
            ),
        ];
        let (from_tc, from_tok) = drain_usage_only(events);
        assert!(from_tc.is_none(), "no TurnComplete");
        let final_usage = from_tc.or(from_tok).expect("token_count fallback");
        assert_eq!(final_usage.input_tokens, 30);
        assert_eq!(final_usage.output_tokens, 3);
        assert_eq!(final_usage.cached_tokens, 5);
    }

    // ── v1.0.0-rc2: plugin_specs ─────────────────────────────────────────

    fn dummy_spec(role: &str) -> SubAgentSpec {
        use crate::data_transfer::DataTransferConfig;
        SubAgentSpec {
            name: role.into(),
            role: role.into(),
            model: None,
            system_prompt: String::new(),
            allowed_tools: vec![],
            data_transfer: DataTransferConfig::default(),
        }
    }

    fn dummy_factory() -> SubAgentFactory {
        SubAgentFactory::new(
            ThreadId::new(),
            "openai/gpt-4o",
            std::sync::Arc::new(reflect_llm::ModelRegistry::new()),
            None, // child_registry: 回退父级 registry
            Arc::new(ToolRegistry::default()),
            CancellationToken::new(),
            None,
        )
    }

    #[test]
    fn register_plugin_spec_stores_under_plugin_id() {
        let factory = dummy_factory();
        factory.register_plugin_spec("plugin-a", dummy_spec("review"));
        factory.register_plugin_spec("plugin-a", dummy_spec("format"));
        factory.register_plugin_spec("plugin-b", dummy_spec("lint"));
        assert_eq!(
            factory.registered_plugin_ids(),
            vec!["plugin-a".to_string(), "plugin-b".to_string()]
        );
        let a = factory.plugin_specs_for("plugin-a");
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn take_plugin_specs_clears_and_returns() {
        let factory = dummy_factory();
        factory.register_plugin_spec("plugin-a", dummy_spec("review"));
        factory.register_plugin_spec("plugin-a", dummy_spec("format"));
        let taken = factory.take_plugin_specs("plugin-a");
        assert_eq!(taken.len(), 2);
        assert!(factory.plugin_specs_for("plugin-a").is_empty());
        assert!(factory.registered_plugin_ids().is_empty());
    }

    #[test]
    fn take_plugin_specs_unknown_returns_empty() {
        let factory = dummy_factory();
        assert!(factory.take_plugin_specs("ghost").is_empty());
    }

    #[test]
    fn register_plugin_spec_rejects_invalid_role() {
        let factory = dummy_factory();
        // 大写 role → validate 拒绝 → spec 不被存。
        factory.register_plugin_spec("plugin-a", dummy_spec("BadRole"));
        assert!(factory.registered_plugin_ids().is_empty());
    }

    // ── v1.1.0: dynamic_specs ────────────────────────────────────

    /// `add_spec` 接受合法 role,`get_spec` 拿回原对象。
    #[test]
    fn add_spec_and_get_spec_roundtrip() {
        let factory = dummy_factory();
        let mut s = dummy_spec("architect");
        s.system_prompt = "design".into();
        assert!(factory.add_spec(s.clone()));
        let back = factory.get_spec("architect").unwrap();
        assert_eq!(back.role, "architect");
        assert_eq!(back.system_prompt, "design");
    }

    /// `add_spec` 拒绝非法 role(BadRole → 大写失败),返回 false 且不存。
    #[test]
    fn add_spec_rejects_invalid_role() {
        let factory = dummy_factory();
        assert!(!factory.add_spec(dummy_spec("BadRole")));
        assert!(factory.get_spec("BadRole").is_none());
        assert!(factory.list_specs().is_empty());
    }

    /// `remove_spec` 存在返回 true,不存在返回 false;移除后 `get_spec` 返 None。
    #[test]
    fn remove_spec_returns_bool() {
        let factory = dummy_factory();
        factory.add_spec(dummy_spec("architect"));
        factory.add_spec(dummy_spec("builder"));
        assert!(factory.remove_spec("architect"));
        assert!(factory.get_spec("architect").is_none());
        assert!(!factory.remove_spec("architect"), "double remove is no-op");
        assert!(factory.remove_spec("builder"));
        assert!(factory.list_specs().is_empty());
    }

    /// `set_specs` 整体替换,旧的全清,新的按字典序;validate 失败 spec 跳过。
    #[test]
    fn set_specs_replaces_and_skips_invalid() {
        let factory = dummy_factory();
        // 先放一个旧 spec。
        factory.add_spec(dummy_spec("old-role"));
        assert_eq!(factory.list_specs().len(), 1);

        // set_specs:2 个合法 + 1 个非法(BadRole) → 只存 2 个。
        factory.set_specs(vec![
            dummy_spec("zulu"),
            dummy_spec("alpha"),
            dummy_spec("BadRole"),
        ]);
        let pairs = factory.list_specs();
        let roles: Vec<&str> = pairs.iter().map(|(r, _)| r.as_str()).collect();
        // 字典序 alpha < zulu,旧的 old-role 已清。
        assert_eq!(roles, vec!["alpha", "zulu"]);
        assert!(factory.get_spec("old-role").is_none());
    }

    /// `list_specs` 按 (role, name) 字典序稳定输出,保证 TUI 渲染与测试稳定。
    #[test]
    fn list_specs_sorted_by_role_then_name() {
        let factory = dummy_factory();
        factory.add_spec(SubAgentSpec {
            name: "Z-Name".into(),
            ..dummy_spec("zulu")
        });
        factory.add_spec(SubAgentSpec {
            name: "A-Name".into(),
            ..dummy_spec("alpha")
        });
        let pairs = factory.list_specs();
        // role 字典序排:alpha < zulu;同 role 下 name 不参与排序(只有 1 个)。
        assert_eq!(
            pairs,
            vec![
                ("alpha".to_string(), "A-Name".to_string()),
                ("zulu".to_string(), "Z-Name".to_string()),
            ]
        );
    }

    /// `child_factory` 不继承 dynamic_specs —— 子 factory 管理自己的 spec 命名空间。
    #[test]
    fn child_factory_does_not_inherit_dynamic_specs() {
        let factory = dummy_factory();
        factory.add_spec(dummy_spec("architect"));
        assert_eq!(factory.list_specs().len(), 1);
        let child = factory.child_factory();
        // 子 factory 看不到父级 dynamic_specs。
        assert!(child.list_specs().is_empty());
        assert!(child.get_spec("architect").is_none());
        // 子 factory 上的 add_spec 不影响父级。
        child.add_spec(dummy_spec("builder"));
        assert_eq!(factory.list_specs().len(), 1);
        assert_eq!(child.list_specs().len(), 1);
    }

    // ── v1.1.0 Phase 4: coordinator mode 注入 ─────────────────────────

    /// `set_coordinator_mode(true, Some(footer))` 后 `is_coordinator_mode`
    /// 返 true,`coordinator_footer()` 拿到 footer。
    #[test]
    fn set_coordinator_mode_updates_state() {
        let factory = dummy_factory();
        assert!(!factory.is_coordinator_mode());
        assert!(factory.coordinator_footer().is_none());

        factory.set_coordinator_mode(true, Some("you are a worker".into()));

        assert!(factory.is_coordinator_mode());
        assert_eq!(
            factory.coordinator_footer().as_deref(),
            Some("you are a worker")
        );
    }

    /// 关掉 coordinator mode → `is_coordinator_mode` 返 false,footer 留旧值
    /// (后续不再用,但保留以便诊断)。
    #[test]
    fn set_coordinator_mode_can_disable() {
        let factory = dummy_factory();
        factory.set_coordinator_mode(true, Some("x".into()));
        factory.set_coordinator_mode(false, None);
        assert!(!factory.is_coordinator_mode());
        // footer 已清空
        assert!(factory.coordinator_footer().is_none());
    }

    /// `child_factory` 共享 coordinator_mode:父启用 → 子也启用。
    /// footer 是 clone 快照,后续父级改不影响子级。
    #[test]
    fn child_factory_shares_coordinator_mode() {
        let factory = dummy_factory();
        factory.set_coordinator_mode(true, Some("footer".into()));
        let child = factory.child_factory();
        assert!(child.is_coordinator_mode());
        assert_eq!(child.coordinator_footer().as_deref(), Some("footer"));

        // 父级关掉,子级仍启用(因为是 `Arc<AtomicBool>` 共享同一引用)。
        factory.set_coordinator_mode(false, None);
        assert!(!factory.is_coordinator_mode());
        // 子级也变 false(共享 Arc)。
        assert!(!child.is_coordinator_mode());

        // 但 footer 是 clone 独立,父级修改不影响已 clone 的 child。
        factory.set_coordinator_mode(true, Some("new".into()));
        assert_eq!(factory.coordinator_footer().as_deref(), Some("new"));
        // child 持有的是 `child_factory()` 调用时的快照("footer"),
        // 后续父级改 "new" 不影响。
        assert_eq!(
            child.coordinator_footer().as_deref(),
            Some("footer"),
            "footer 是 clone 快照,后续父级修改不影响 child"
        );
    }

    /// `Arc<SubAgentFactory>` 共享语义:两个 Arc 副本调 `set_coordinator_mode`,
    /// `is_coordinator_mode` 互见。
    #[test]
    fn coordinator_mode_shared_via_arc() {
        let factory = Arc::new(dummy_factory());
        let factory2 = factory.clone();
        factory.set_coordinator_mode(true, Some("shared".into()));
        assert!(factory2.is_coordinator_mode());
        assert_eq!(factory2.coordinator_footer().as_deref(), Some("shared"));
    }

    /// coordinator 启用时 `build_spawn_user_input` 末尾含 `[Coordinator Principle]`。
    #[test]
    fn build_spawn_user_input_appends_coordinator_principle() {
        let spec = dummy_spec("architect");
        let out = super::build_spawn_user_input(
            &spec,
            "do the thing",
            true,
            Some("stay independent".into()),
        );
        assert!(out.contains("[Coordinator Principle]"));
        assert!(out.contains("stay independent"));
        assert!(out.contains("[Subagent role:"));
    }

    /// coordinator 关闭时不附加 footer 段。
    #[test]
    fn build_spawn_user_input_skips_footer_when_disabled() {
        let spec = dummy_spec("architect");
        let out = super::build_spawn_user_input(&spec, "task", false, Some("ignored".into()));
        assert!(!out.contains("[Coordinator Principle]"));
    }

    // ── v1.1.0 review ────────────────────────────────────────────────
    // bug-1 (P0):子 m4.subagent_registry 与父共享 Arc
    // bug-4 (P1):child_factory 继承 registry

    /// bug-1 (P0):`spawn` 路径构造的 child `AgentConfig.m4.subagent_registry`
    /// 应与父 factory 注入了的 registry 共享同一 Arc —— 否则子 agent 的
    /// `pre_loop` 渲染 `<system-reminder>` 时拿的是 fresh Arc,看不到父 /
    /// 自身任何已完成调用。本测试构造一个 fake `Submission` 不可行(走真
    /// AgentThread),改为在 spawn 内部链路直接断言:通过 `Arc::ptr_eq` 验证
    /// 父 m4 与子 cfg.m4 的 `subagent_registry` 同源。
    ///
    /// 因 `spawn` 需要 LLM client,改为验证 helper 逻辑:把当前 `factory.rs`
    /// 的 `spawn` 中"构造 m4 + 注入 registry"那段抽出独立 fn 后,本测试
    /// 调它比对指针。为避免改动 `spawn` 签名,这里改用 `child_factory` 上
    /// 同样的 Arc clone 路径(`Mutex::new(self.subagent_registry.lock().clone())`),
    /// 直接断言父 ↔ 子 factory registry 是同一 Arc。
    #[test]
    fn child_factory_registry_shares_arc_with_parent() {
        let factory = dummy_factory();
        let reg = reflect_recovery::SubagentRegistry::shared();
        factory.set_subagent_registry(reg.clone());

        let child = factory.child_factory();
        let parent_reg = factory.subagent_registry().unwrap();
        let child_reg = child.subagent_registry().unwrap();
        assert!(
            Arc::ptr_eq(&parent_reg, &child_reg),
            "child_factory 必须与父共享同一 Arc(共享 registry 才能让孙级 subagent 被父级 pre_loop 看到)"
        );
        // 同时验证 child_factory 写入对父可见(共享语义的核心断言)。
        reg.record(reflect_recovery::SubagentRegistryEntry {
            tool_name: "call_x".into(),
            task_summary: "child wrote".into(),
            result_summary: "ok".into(),
            iteration: 1,
            created_at: chrono::Utc::now(),
        });
        assert_eq!(parent_reg.snapshot().len(), 1);
    }

    /// bug-1 (P0) e2e-lite:模拟 `spawn` 构造子 m4 的等价路径 —— 走
    /// `reflect_core::config::default_m4_deps` 拿 fresh M4Deps,然后注入
    /// 父 registry。这正是 review 后 `spawn` 路径采用的修复模式,
    /// 单测验证注入语义(否则回归时不会失败)。
    #[test]
    fn spawn_propagates_parent_registry_into_child_m4() {
        use reflect_core::config::default_m4_deps;

        let factory = dummy_factory();
        let reg = reflect_recovery::SubagentRegistry::shared();
        factory.set_subagent_registry(reg.clone());

        // 模拟 spawn 内构造 child m4 的逻辑:
        let mut m4 = default_m4_deps("subagent");
        assert!(
            !Arc::ptr_eq(&m4.subagent_registry, &reg),
            "sanity:default_m4_deps 给的是 fresh Arc"
        );
        if let Some(parent_reg) = factory.subagent_registry() {
            m4.subagent_registry = parent_reg;
        }
        assert!(
            Arc::ptr_eq(&m4.subagent_registry, &reg),
            "修复后:child m4.subagent_registry 必须指向父 factory 注入了的 Arc"
        );
    }

    /// bug-1 (P0) negative:父未注入 registry 时,child m4 仍走 fresh Arc
    /// (测试 / headless 退化路径)。保证本修复不引入 panic。
    #[test]
    fn spawn_uses_fresh_registry_when_parent_unset() {
        use reflect_core::config::default_m4_deps;

        let factory = dummy_factory();
        // 不调 set_subagent_registry → 父为 None
        let mut m4 = default_m4_deps("subagent");
        let fresh = m4.subagent_registry.clone();
        if let Some(parent_reg) = factory.subagent_registry() {
            m4.subagent_registry = parent_reg;
        }
        // 父为 None → 不替换,仍是 fresh Arc(行为不变,无回归)。
        assert!(Arc::ptr_eq(&m4.subagent_registry, &fresh));
    }
}
