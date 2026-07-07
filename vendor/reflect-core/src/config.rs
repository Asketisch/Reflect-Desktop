//! Runtime configuration for an `AgentThread`.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use reflect_agent_def::AgentDefinition;
use reflect_compact::{Compactor, CompactorConfig};
use reflect_llm::RoutingPolicy;
use reflect_memory::{FileMemoryStore, MemoryStore};
use reflect_notes::NoteStore;
use reflect_prompt::PromptBuilder;
use reflect_protocol::{PermissionMode, ReasoningEffortMirror, RolloutRecorder};
use reflect_recovery::{ActiveFileRecovery, SubagentRegistry};
use reflect_skills::SkillsCatalog;
use reflect_tools::SessionWorktreeState;
use std::collections::HashMap;

/// Environment variable that overrides the default compaction trigger
/// threshold. Matches claw's `CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS`.
pub const AUTO_COMPACT_INPUT_TOKENS_ENV: &str = "REFLECT_AUTO_COMPACT_INPUT_TOKENS";

/// v1.2 P1-12:Environment variable that sets a hard session-level token
/// budget. When cumulative session usage (input + output) reaches this
/// value, the turn ends with `TurnStatus::TokenBudgetExceeded`.
/// Priority: env `REFLECT_TOKEN_BUDGET` > TOML `[token_budget].session_total_tokens` > None.
pub const TOKEN_BUDGET_ENV: &str = "REFLECT_TOKEN_BUDGET";

/// Resolve the session token budget in priority order:
/// env `REFLECT_TOKEN_BUDGET` > `toml_budget` > `None`.
///
/// Reads the env at call time so tests can flip it between cases. Production
/// callers (`reflect-exec::bootstrap`) snapshot the value at startup. Mirrors
/// [`trigger_tokens_from_env`] (compaction) but returns `Option<u64>` since
/// "no budget" is a valid default (only `max_iterations` bounds the loop).
pub fn token_budget_from_env(toml_budget: Option<u64>) -> Option<u64> {
    if let Some(env_val) = std::env::var(TOKEN_BUDGET_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        return Some(env_val);
    }
    toml_budget
}

/// Default trigger threshold (matches claw's `10000`). Reads the env var at
/// call time so tests can flip it between cases; production callers
/// (`reflect-exec::bootstrap_m5`) snapshot the value at startup.
pub fn trigger_tokens_from_env() -> u32 {
    std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000)
}

/// Build the `CompactorConfig` for the session by resolving the trigger
/// threshold in priority order: env `REFLECT_AUTO_COMPACT_INPUT_TOKENS`
/// > TOML `trigger_tokens` > `CompactorConfig::default()` (= 10 000).
///
/// `toml_trigger_tokens` is the value of `ReflectConfig.compact.trigger_tokens`
/// from `~/.reflect/config.toml`. Pass `None` if the section is absent.
/// `CompactorConfig::default()` is the single source of truth for the
/// other tunables (`microcompact_ratio`, `keep_recent_*`, `target_ratio`,
/// `summarize_after`); we override only `trigger_tokens`.
///
/// This function reads the env at call time so tests can flip
/// `REFLECT_AUTO_COMPACT_INPUT_TOKENS` between cases (M7 parity: the
/// `std::env::set_var` races are serialized by the per-module env lock in
/// the test suite).
pub fn compactor_config_from_env_and_toml(toml_trigger_tokens: Option<u32>) -> CompactorConfig {
    let mut cfg = CompactorConfig::default();
    if let Some(env_val) = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        cfg.trigger_tokens = env_val;
    } else if let Some(toml_val) = toml_trigger_tokens {
        cfg.trigger_tokens = toml_val;
    }
    cfg
}

#[derive(Clone)]
pub struct AgentConfig {
    /// Default model spec (`"openai/gpt-4o"`, `"anthropic/claude-3-5-sonnet-latest"`, …)。
    ///
    /// v0.2.2 起改为 `Arc<RwLock<String>>`:`reflect-exec` 的热重载 task 会在
    /// `~/.reflect/config.toml` 变更时调 `set_model()` 写入新值,后续 turn
    /// 通过 [`Self::current_model`] 读到最新 spec。`Clone` 走 `Arc`,所有
    /// 副本共享同一把锁 —— 见 `submission_loop` 把 `cfg` 移到 `AgentThread`
    /// 之后多处持有 `cfg.clone()` 的语义不变。
    pub model: Arc<RwLock<String>>,
    /// v1.x Plan mode:会话级 `PermissionMode` 状态机。
    ///
    /// 同样用 `Arc<RwLock<…>>` 镜像 `model` 的热重载 pattern:`/plan`
    /// slash 或 `EnterPlanModeTool` 触发后由 `submission_loop` 调
    /// [`Self::set_permission_mode`] 写入新值,所有 hook 引擎(包括
    /// `PlanModeGate`)和 tool queue 通过 [`Self::permission_mode`]
    /// 读到最新 mode。默认 `PermissionMode::Auto`(普通执行模式)。
    pub permission_mode: Arc<RwLock<PermissionMode>>,
    /// Workspace root — tools operate relative to this path.
    ///
    /// v1.x Git worktree:`EnterWorktreeTool` / `ExitWorktreeTool` 通过
    /// [`Self::set_workspace`] 热切换;`ToolContext.workspace` 持有同一把
    /// `RwLock`,后续 tool 调用立刻读到新路径。
    pub workspace: Arc<RwLock<PathBuf>>,
    /// 当前 worktree 隔离会话;`None` 表示未进入 worktree。
    pub worktree: Arc<RwLock<Option<SessionWorktreeState>>>,
    /// Cooperative cancellation (Ctrl-C, Op::Interrupt, etc.).
    pub cancel: CancellationToken,
    /// M4 dependencies. All are `None` in tests; `reflect-exec` populates
    /// them at startup. When `None`, the submission loop falls back to
    /// no-op / empty implementations so existing tests keep working.
    pub m4: Option<M4Deps>,
    /// M6: install per-turn `ApprovalGate`s so `Prompt`-permission tools
    /// route through `EventMsg::ApprovalRequest`. Defaults to `false` for
    /// the headless `reflect-exec` JSONL path; the TUI / lib facade
    /// flip it on when constructing the thread. Also respects the
    /// `REFLECT_APPROVALS=1` env var as a fallback for advanced use.
    pub approvals: bool,
    /// v1.0 多 Provider 路由:角色 → spec slot 的路由策略。
    ///
    /// `model_call` 入口用 `policy.resolve(Role::Main)` 拿到 spec,失败
    /// 时由 `ModelRegistry::next_for` 在 pool 内自动切下一个 credential。
    /// 默认 `Arc::new(RoutingPolicy::default())`;`reflect-exec::bootstrap_m4`
    /// 在读到 `[routing]` 段后调 [`Self::set_policy`] 替换为实际策略。
    pub policy: Arc<RoutingPolicy>,
    /// v1.x S4:`/effort low|medium|high` 透传槽。`model_call` 入口读
    /// 当前值构造 `ChatRequest::thinking = ThinkingConfig::OpenAIReasoning
    /// { effort: llm_effort(mirror) }`,下一轮 LLM 调用立刻生效。
    ///
    /// 与 `permission_mode` 同 pattern:`submission_loop` 在收到
    /// `Op::SetEffort` 后调 [`Self::set_effort`] 写入;`Clone` 后多副本
    /// 共享同一把 `RwLock`。默认 `Low`,与 Anthropic / OpenAI 默认
    /// reasoning 强度一致(避免 "未设置 = 不思考" 的歧义)。
    pub effort: Arc<RwLock<ReasoningEffortMirror>>,
    /// S5a:可选 permission resolver,注入 `ApprovalGate::with_state`。
    /// TUI / reflect-exec 从 `FilePermissionStore` 构造;测试 / headless
    /// 默认 `None`(modal-only 行为)。
    pub permission_resolver: Option<Arc<dyn reflect_permissions::PermissionResolver>>,
    /// v1.2 P1-12:会话级累计 token 用量(跨所有 turn 累加,镜像 `model` /
    /// `effort` 的 `Arc<RwLock<…>>` 共享模式)。`model_call` 每次调用后
    /// 累加 `_usage`;`get_context_remaining` 工具 / 预算检查都读这把锁。
    /// 与 `AgentState.total_usage`(单 turn 累加、每 turn reset)正交。
    pub session_usage: Arc<RwLock<reflect_protocol::TokenUsage>>,
    /// v1.2 P1-12:会话级 token 预算硬上限。`None` = 仅靠 `max_iterations`;
    /// `Some(n)` = `session_usage.total_tokens >= n` 时终止当前 turn
    /// (`TurnStatus::TokenBudgetExceeded`)。`reflect-exec` 启动期从
    /// `[token_budget].session_total_tokens` / env 解析填入。用 `Arc<RwLock>`
    /// 包裹(同 `model` / `effort` pattern),让 `handle_reload` 热重载后
    /// 下一轮 `model_call` 通过共享句柄读到新值。
    pub token_budget: Arc<RwLock<Option<u64>>>,
    /// v1.2 P1-12:当前 model 的上下文窗口大小(token)。引擎在
    /// `SessionConfigured` 时用 `reflect_llm::context_window_for` 算出后写入
    /// (热重载切 model 后刷新)。`get_context_remaining` 工具读它做分母。
    pub context_window_size: Arc<RwLock<Option<u32>>>,
    /// v1.2 P1-12(已有-B):`/compact` 手动触发标志。`Op::Compact` 设置
    /// `true`,下一个 turn 的 `pre_loop` 据此强制运行 compactor(无视
    /// trigger_tokens 阈值),压缩后清零。共享 `Arc<RwLock>` 让
    /// `submission_loop` 的 `Op::Compact` 分支与 `pre_loop` 节点通信。
    pub force_compact_next: Arc<RwLock<bool>>,
}

/// Bag of M4 dependencies. Each field is independently optional so
/// production bootstrap can populate them incrementally.
#[derive(Clone)]
pub struct M4Deps {
    pub compactor: Arc<Compactor>,
    pub memory: Arc<dyn MemoryStore>,
    pub skills: Arc<SkillsCatalog>,
    pub prompt_builder: Arc<Mutex<PromptBuilder>>,
    pub active_agent_def: Arc<AgentDefinition>,
    /// M5: optional persistence sink for [`RolloutRecord`]s. `None` in
    /// tests; `reflect-exec` wires a `JsonlRolloutWriter` at startup.
    pub recorder: Option<Arc<dyn RolloutRecorder>>,
    /// v1.1.0 Phase 6 P0:Session memory 笔记存储(FIFO 30 + JSONL
    /// 落盘)。`pre_loop` 调 `as_meta_message()` 把当前队列渲染成
    /// `<system-reminder>` 注入到 LLM。
    pub note_store: Arc<dyn NoteStore>,
    /// v1.1.0 Phase 6 P0:post-compact 活跃文件恢复。`pre_loop` 在压缩
    /// 触发后调 `recover(&messages)`,把最近 write / edit 过的文件
    /// 内容(50k token 预算,10 文件上限)注入到 `<system-reminder>`。
    pub file_recovery: Arc<ActiveFileRecovery>,
    /// v1.1.0 Phase 6 P0:已完成子代理调用注册表。`CallSubAgentTool`
    /// 写,`pre_loop` 读 + 渲染为 `[已完成的子代理调用记录]` 防止 LLM
    /// 重复 spawn。FIFO cap=32,跨 turn 共享在 `M4Deps` 上。
    pub subagent_registry: Arc<SubagentRegistry>,
}

impl std::fmt::Debug for M4Deps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("M4Deps")
            .field("compactor", &"<dyn Compactor>")
            .field("memory", &"<dyn MemoryStore>")
            .field("skills", &self.skills)
            .field("prompt_builder", &"<PromptBuilder>")
            .field("active_agent_def", &self.active_agent_def.name)
            .field(
                "recorder",
                &self.recorder.as_ref().map(|_| "<dyn RolloutRecorder>"),
            )
            .field("note_store", &"<dyn NoteStore>")
            .field("file_recovery", &self.file_recovery)
            .field("subagent_registry", &self.subagent_registry)
            .finish()
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::new("", "")
    }
}

impl AgentConfig {
    pub fn new(model: impl Into<String>, workspace: impl Into<PathBuf>) -> Self {
        Self {
            model: Arc::new(RwLock::new(model.into())),
            permission_mode: Arc::new(RwLock::new(PermissionMode::Auto)),
            workspace: Arc::new(RwLock::new(workspace.into())),
            worktree: Arc::new(RwLock::new(None)),
            cancel: CancellationToken::new(),
            m4: None,
            approvals: false,
            policy: Arc::new(RoutingPolicy::default()),
            effort: Arc::new(RwLock::new(ReasoningEffortMirror::Low)),
            permission_resolver: None,
            session_usage: Arc::new(RwLock::new(reflect_protocol::TokenUsage::default())),
            token_budget: Arc::new(RwLock::new(None)),
            context_window_size: Arc::new(RwLock::new(None)),
            force_compact_next: Arc::new(RwLock::new(false)),
        }
    }

    /// 当前激活的 model spec 快照(读 `RwLock` 后 clone)。读侧唯一入口;
    /// `pre_loop` / `model_call` / 提交循环等都在这里取最新值,以便
    /// 热重载后下一个 turn 立刻用上新 model。
    pub fn current_model(&self) -> String {
        self.model.read().clone()
    }

    /// 写入新 model spec。仅供 `reflect-exec::handle_reload` 在 TOML 热重载
    /// 检测到 model 变更时调用 —— 库用户想在会话内切 model 也走这条
    /// `reflect-config` reload 路径,直接调用会绕过 diff / event 通知。
    pub fn set_model(&self, new_spec: impl Into<String>) {
        *self.model.write() = new_spec.into();
    }

    /// 当前会话的 `PermissionMode` 快照。
    ///
    /// 默认 `PermissionMode::Auto`(普通执行模式);TUI `/plan` slash 或
    /// `EnterPlanModeTool` 触发后由 `submission_loop` 切到 `Plan`,
    /// 期间 `PlanModeGate` hook 会 blanket-deny 写工具。
    pub fn permission_mode(&self) -> PermissionMode {
        *self.permission_mode.read()
    }

    /// 写入新 `PermissionMode`。仅供 `submission_loop` 在用户批准
    /// `EnterPlanModeTool` / `ExitPlanModeTool` 后调用;直接调用会
    /// 绕过 `EventMsg::PermissionModeChanged` 通知路径。
    pub fn set_permission_mode(&self, new_mode: PermissionMode) {
        *self.permission_mode.write() = new_mode;
    }

    /// v1.x S4:当前会话的 `ReasoningEffortMirror` 快照。
    ///
    /// 默认 `Low`;`submission_loop` 在收到 `Op::SetEffort` 后调
    /// [`Self::set_effort`] 写入新值,`model_call` 在构造 `ChatRequest`
    /// 时调 [`Self::current_effort`] 取最新值。
    pub fn current_effort(&self) -> ReasoningEffortMirror {
        *self.effort.read()
    }

    /// v1.x S4:写入新 reasoning effort。仅供 `submission_loop` 在收到
    /// `Op::SetEffort` 后调用;直接调用会绕过 `tracing::info!` 审计行。
    pub fn set_effort(&self, new_effort: ReasoningEffortMirror) {
        *self.effort.write() = new_effort;
    }

    /// 当前 workspace 根路径快照。
    pub fn current_workspace(&self) -> PathBuf {
        self.workspace.read().clone()
    }

    /// 热切换 workspace。`EnterWorktreeTool` / `ExitWorktreeTool` 在
    /// 用户审批通过后调用;与 `ToolContext` 共享 `Arc<RwLock<…>>`。
    pub fn set_workspace(&self, path: impl Into<PathBuf>) {
        *self.workspace.write() = path.into();
    }

    /// 读取 worktree 会话状态。
    pub fn worktree_state(&self) -> Option<SessionWorktreeState> {
        self.worktree.read().clone()
    }

    /// 写入 worktree 会话状态。
    pub fn set_worktree_state(&self, state: Option<SessionWorktreeState>) {
        *self.worktree.write() = state;
    }

    /// 构造 `ToolContext` 时共享 workspace / worktree / session_usage /
    /// token_budget 句柄。
    pub fn shared_tool_context(&self) -> reflect_tools::ToolContext {
        let mut ctx = reflect_tools::ToolContext::with_shared(
            Arc::clone(&self.workspace),
            Arc::clone(&self.worktree),
        );
        // v1.2 P1-12:共享会话用量 / 预算 / 上下文窗口句柄,让
        // `get_context_remaining` 工具读到 `model_call` 实时累加的值
        // (同 `workspace` 共享 pattern)。
        ctx.session_usage = Arc::clone(&self.session_usage);
        ctx.token_budget = Arc::clone(&self.token_budget);
        ctx.context_window_size = Arc::clone(&self.context_window_size);
        ctx
    }

    /// Set the M4 dependencies. Used by `reflect-exec` at startup.
    pub fn with_m4(mut self, m4: M4Deps) -> Self {
        self.m4 = Some(m4);
        self
    }

    /// Enable per-turn `ApprovalGate`s. The TUI / lib calls this; the
    /// headless exec driver leaves it off.
    pub fn with_approvals(mut self, on: bool) -> Self {
        self.approvals = on;
        self
    }

    /// v1.x Plan mode:在构造时设置初始 `PermissionMode`(用于 `--plan-mode` CLI 旗标)。
    pub fn with_initial_permission_mode(self, mode: PermissionMode) -> Self {
        *self.permission_mode.write() = mode;
        self
    }

    /// S5a:挂载 `StorePermissionResolver`(与 TUI `/permissions` 同源 store)。
    pub fn with_permission_resolver(
        mut self,
        resolver: Arc<dyn reflect_permissions::PermissionResolver>,
    ) -> Self {
        self.permission_resolver = Some(resolver);
        self
    }

    /// 用外部 `CancellationToken` 替换默认 token —— 集成测试共享 cancel
    /// 用。生产代码不需要这条路径 (`reflect-exec` 自己 wire Ctrl-C)。
    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// 构造时一次性塞入 `RoutingPolicy`(给 `bootstrap_m4` 启动期用)。
    /// v1.0 Phase 1:`policy` 字段构造后不可变;热重载在 Phase 3 接
    /// `Arc<ArcSwap<RoutingPolicy>>` 后另开 `set_policy()` 方法。
    pub fn with_policy(mut self, policy: Arc<RoutingPolicy>) -> Self {
        self.policy = policy;
        self
    }

    /// v1.2 P1-12:构造时设置会话 token 预算上限。
    pub fn with_token_budget(self, budget: Option<u64>) -> Self {
        *self.token_budget.write() = budget;
        self
    }

    /// v1.2 P1-12:热重载时更新会话 token 预算上限。`handle_reload`
    /// 在检测到 `[token_budget]` 段变更时调用;`Clone` 后多副本共享
    /// 同一把 RwLock,下一轮 `model_call` 读最新值(与 `set_model` /
    /// `set_effort` 同 pattern)。
    pub fn set_token_budget(&self, budget: Option<u64>) {
        *self.token_budget.write() = budget;
    }

    /// v1.2 P1-12:会话 token 预算快照(`None` = 无上限)。
    pub fn current_token_budget(&self) -> Option<u64> {
        *self.token_budget.read()
    }

    /// v1.2 P1-12(已有-B):设置 `/compact` 强制标志(由 `Op::Compact` 调)。
    pub fn request_force_compact(&self) {
        *self.force_compact_next.write() = true;
    }

    /// v1.2 P1-12(已有-B):读 + 清零强制标志(由 `pre_loop` 调,返回是否该
    /// 强制压缩本轮)。
    pub fn take_force_compact(&self) -> bool {
        let mut g = self.force_compact_next.write();
        let v = *g;
        *g = false;
        v
    }

    /// v1.2 P1-12:会话级累计 token 用量快照(读 `RwLock` 后 clone)。
    /// `get_context_remaining` 工具与预算检查的读侧入口。
    pub fn current_session_usage(&self) -> reflect_protocol::TokenUsage {
        self.session_usage.read().clone()
    }

    /// v1.2 P1-12:把单次 model_call 的 `_usage` 累加进会话级总量。
    /// `model_call` 每次调用后调;`Clone` 后多副本共享同一把 RwLock,
    /// 与单 turn 的 `AgentState.total_usage`(每 turn reset)正交。
    pub fn add_session_usage(&self, delta: &reflect_protocol::TokenUsage) {
        let mut s = self.session_usage.write();
        s.input_tokens = s.input_tokens.saturating_add(delta.input_tokens);
        s.output_tokens = s.output_tokens.saturating_add(delta.output_tokens);
        s.cached_tokens = s.cached_tokens.saturating_add(delta.cached_tokens);
        s.cache_write_tokens = s.cache_write_tokens.saturating_add(delta.cache_write_tokens);
        s.total_tokens = s.total_tokens.saturating_add(delta.total_tokens);
    }

    /// v1.2 P1-12:会话预算是否已耗尽。`None` budget 永不耗尽。
    pub fn session_budget_exceeded(&self) -> bool {
        match *self.token_budget.read() {
            Some(limit) => self.session_usage.read().total_tokens as u64 >= limit,
            None => false,
        }
    }
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("model", &self.current_model())
            .field("permission_mode", &self.permission_mode())
            .field("workspace", &self.current_workspace())
            .field("worktree", &self.worktree_state())
            .field("approvals", &self.approvals)
            .field(
                "permission_resolver",
                &self
                    .permission_resolver
                    .as_ref()
                    .map(|_| "<dyn PermissionResolver>"),
            )
            .finish()
    }
}

/// Helper for tests: build a no-op `M4Deps` with the given agent name.
/// Uses empty `Compactor` (no-op) + empty `FileMemoryStore` (writes to a
/// tempdir or scratch dir) + empty `SkillsCatalog` + default
/// `PromptBuilder` + a default `AgentDefinition`.
pub fn default_m4_deps(agent_name: &str) -> M4Deps {
    use reflect_compact::{CompactorConfig, Summarizer};
    use std::sync::Arc as StdArc;
    struct NoopSummarizer;
    #[async_trait::async_trait]
    impl Summarizer for NoopSummarizer {
        async fn summarize_full(
            &self,
            _msgs: &[reflect_llm::ChatMessage],
        ) -> Result<String, reflect_compact::SummarizerError> {
            Err(reflect_compact::SummarizerError::Cancelled)
        }
        async fn summarize_recent(
            &self,
            _msgs: &[reflect_llm::ChatMessage],
            _prev: Option<&str>,
        ) -> Result<String, reflect_compact::SummarizerError> {
            Err(reflect_compact::SummarizerError::Cancelled)
        }
    }
    let compactor = Arc::new(Compactor::new(
        CompactorConfig {
            summarize_after: false, // tests don't want a real LLM call
            ..Default::default()
        },
        StdArc::new(NoopSummarizer),
    ));
    // Use a unique tempdir for memory to avoid cross-test pollution.
    let tmp = std::env::temp_dir().join(format!("reflect-test-mem-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&tmp);
    let memory: Arc<dyn MemoryStore> = Arc::new(FileMemoryStore::new(&tmp, &tmp));
    let skills = Arc::new(SkillsCatalog::new());
    let prompt_builder = Arc::new(Mutex::new(PromptBuilder::new()));
    let note_store: Arc<dyn reflect_notes::NoteStore> =
        Arc::new(reflect_notes::InMemoryNoteStore::new());
    let file_recovery = Arc::new(reflect_recovery::ActiveFileRecovery::new(Arc::from(tmp)));
    let subagent_registry = reflect_recovery::SubagentRegistry::shared();
    let mut def = AgentDefinition::default();
    #[allow(clippy::field_reassign_with_default)] // pre-M5: clearer than struct-literal
    {
        def.name = agent_name.to_string();
        def.description = format!("test agent {agent_name}");
        def.system_prompt = "You are a test agent.".into();
    }
    M4Deps {
        compactor,
        memory,
        skills,
        prompt_builder,
        active_agent_def: Arc::new(def),
        recorder: None,
        note_store,
        file_recovery,
        subagent_registry,
    }
}

// Suppress unused import warning for HashMap in builds that don't
// pull in additional future fields.
#[allow(dead_code)]
fn _unused_hashmap_marker(_: HashMap<String, String>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    /// `std::env::set_var` is `unsafe` in Rust 2024 (not thread-safe with
    /// concurrent `env::var` reads). All tests in this module serialize
    /// through this lock to avoid data races during parallel test execution.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn trigger_tokens_from_env_default() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
        }
        assert_eq!(trigger_tokens_from_env(), 10_000);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        }
    }

    #[test]
    fn trigger_tokens_from_env_override() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, "12345");
        }
        assert_eq!(trigger_tokens_from_env(), 12_345);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
            }
        }
    }

    #[test]
    fn trigger_tokens_from_env_malformed_returns_default() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, "not-a-number");
        }
        assert_eq!(trigger_tokens_from_env(), 10_000);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
            }
        }
    }

    // ── M8 P0b: compactor_config_from_env_and_toml ──────────────────────

    #[test]
    fn compactor_config_default_when_no_env_no_toml() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
        }
        let cfg = compactor_config_from_env_and_toml(None);
        assert_eq!(cfg.trigger_tokens, 10_000);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        }
    }

    #[test]
    fn compactor_config_uses_toml_when_env_absent() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
        }
        let cfg = compactor_config_from_env_and_toml(Some(5_000));
        assert_eq!(cfg.trigger_tokens, 5_000);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        }
    }

    #[test]
    fn compactor_config_env_wins_over_toml() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, "200");
        }
        let cfg = compactor_config_from_env_and_toml(Some(99_999));
        assert_eq!(cfg.trigger_tokens, 200, "env must override TOML");
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
            }
        }
    }

    #[test]
    fn compactor_config_env_malformed_falls_through_to_toml() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(AUTO_COMPACT_INPUT_TOKENS_ENV).ok();
        unsafe {
            std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, "not-a-number");
        }
        let cfg = compactor_config_from_env_and_toml(Some(7_777));
        assert_eq!(
            cfg.trigger_tokens, 7_777,
            "malformed env should fall through to TOML"
        );
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(AUTO_COMPACT_INPUT_TOKENS_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(AUTO_COMPACT_INPUT_TOKENS_ENV);
            }
        }
    }

    // ── v0.2.2 热重载切 model ──────────────────────────────────────────

    /// `current_model` 读初始值。
    #[test]
    fn agent_config_current_model_returns_initial_value() {
        let cfg = AgentConfig::new("anthropic/claude-3-5-sonnet-latest", "/tmp");
        assert_eq!(cfg.current_model(), "anthropic/claude-3-5-sonnet-latest");
    }

    /// `set_model` 写入新值,`current_model` 立刻读到。
    #[test]
    fn agent_config_set_model_updates_current_model() {
        let cfg = AgentConfig::new("anthropic/claude-3-5-sonnet-latest", "/tmp");
        cfg.set_model("openai/gpt-4o");
        assert_eq!(cfg.current_model(), "openai/gpt-4o");
    }

    /// `Clone` 后两副本共享同一把 RwLock —— 写其中一个,另一个也看见。
    /// 这是热重载能贯穿 `submission_loop` 多处 `cfg.clone()` 的基础。
    #[test]
    fn agent_config_clone_shares_rwlock() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        let cfg2 = cfg.clone();
        cfg.set_model("openai/gpt-4o");
        assert_eq!(
            cfg2.current_model(),
            "openai/gpt-4o",
            "clone must observe writes via shared RwLock"
        );
    }

    /// `Debug` 输出包含 model spec —— `ReflectBuilder` 的 Debug 用例和
    /// snapshot 测试都依赖这一行。
    #[test]
    fn agent_config_debug_includes_model_spec() {
        let cfg = AgentConfig::new("anthropic/claude-3-5-sonnet-latest", "/tmp");
        let dbg = format!("{cfg:?}");
        assert!(
            dbg.contains("anthropic/claude-3-5-sonnet-latest"),
            "Debug must include model spec, got: {dbg}"
        );
    }

    // ── v1.x Plan mode: permission_mode 热重载 ─────────────────────────────

    /// 默认 `permission_mode` 必须是 `Auto`(普通执行模式)。
    #[test]
    fn agent_config_permission_mode_defaults_to_auto() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        assert_eq!(cfg.permission_mode(), PermissionMode::Auto);
    }

    /// `set_permission_mode` 写入新值,`permission_mode()` 立即读到。
    #[test]
    fn agent_config_set_permission_mode_updates_current() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        cfg.set_permission_mode(PermissionMode::Plan);
        assert_eq!(cfg.permission_mode(), PermissionMode::Plan);
        cfg.set_permission_mode(PermissionMode::Prompt);
        assert_eq!(cfg.permission_mode(), PermissionMode::Prompt);
    }

    /// `Clone` 后两副本共享同一把 RwLock —— 镜像 model 的 pattern,
    /// 让 `submission_loop` 多处持有 `cfg.clone()` 后,Plan 模式切换对所有
    /// 副本都可见。
    #[test]
    fn agent_config_clone_shares_permission_mode_rwlock() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        let cfg2 = cfg.clone();
        cfg.set_permission_mode(PermissionMode::Plan);
        assert_eq!(
            cfg2.permission_mode(),
            PermissionMode::Plan,
            "clone 必须能观察到 permission_mode 写入"
        );
    }

    /// `with_initial_permission_mode` 构造器便捷设置(给 `--plan-mode` CLI 旗标用)。
    #[test]
    fn agent_config_with_initial_permission_mode() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp")
            .with_initial_permission_mode(PermissionMode::Plan);
        assert_eq!(cfg.permission_mode(), PermissionMode::Plan);
    }

    // ── v1.x S4:effort 热重载 ──────────────────────────────────────

    /// 默认 effort = Low(Anthropic / OpenAI 默认 reasoning 强度)。
    #[test]
    fn agent_config_effort_defaults_to_low() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        assert_eq!(cfg.current_effort(), ReasoningEffortMirror::Low);
    }

    /// `set_effort` 写入新值,`current_effort()` 立即读到。
    #[test]
    fn agent_config_set_effort_updates_current() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        cfg.set_effort(ReasoningEffortMirror::High);
        assert_eq!(cfg.current_effort(), ReasoningEffortMirror::High);
        cfg.set_effort(ReasoningEffortMirror::Medium);
        assert_eq!(cfg.current_effort(), ReasoningEffortMirror::Medium);
    }

    /// `Clone` 后两副本共享同一把 RwLock —— 镜像 model / permission_mode
    /// 的 pattern,让 `submission_loop` 多处持有 `cfg.clone()` 后,
    /// `/effort` 切换对所有副本都可见。
    #[test]
    fn agent_config_clone_shares_effort_rwlock() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        let cfg2 = cfg.clone();
        cfg.set_effort(ReasoningEffortMirror::High);
        assert_eq!(
            cfg2.current_effort(),
            ReasoningEffortMirror::High,
            "clone 必须能观察到 effort 写入"
        );
    }

    /// `Clone` 后两副本共享 workspace RwLock —— worktree 热切换对所有
    /// `cfg.clone()` 持有者可见。
    #[test]
    fn agent_config_clone_shares_workspace_rwlock() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp/a");
        let cfg2 = cfg.clone();
        cfg.set_workspace("/tmp/b");
        assert_eq!(cfg2.current_workspace(), PathBuf::from("/tmp/b"));
    }

    /// worktree 状态默认可为空。
    #[test]
    fn agent_config_worktree_defaults_to_none() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        assert!(cfg.worktree_state().is_none());
    }

    // ── v1.2 P1-12: token budget ───────────────────────────────────────

    /// `token_budget_from_env`:无 env 无 toml → `None`(默认不设预算)。
    #[test]
    fn token_budget_from_env_default_none() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(TOKEN_BUDGET_ENV).ok();
        unsafe {
            std::env::remove_var(TOKEN_BUDGET_ENV);
        }
        assert_eq!(token_budget_from_env(None), None);
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(TOKEN_BUDGET_ENV, p);
            }
        }
    }

    /// `token_budget_from_env`:toml 有值、env 无 → 用 toml 值。
    #[test]
    fn token_budget_from_env_uses_toml_when_env_absent() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(TOKEN_BUDGET_ENV).ok();
        unsafe {
            std::env::remove_var(TOKEN_BUDGET_ENV);
        }
        assert_eq!(token_budget_from_env(Some(500_000)), Some(500_000));
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(TOKEN_BUDGET_ENV, p);
            }
        }
    }

    /// `token_budget_from_env`:env 优先于 toml。
    #[test]
    fn token_budget_from_env_wins_over_toml() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(TOKEN_BUDGET_ENV).ok();
        unsafe {
            std::env::set_var(TOKEN_BUDGET_ENV, "999");
        }
        assert_eq!(
            token_budget_from_env(Some(500_000)),
            Some(999),
            "env must override TOML"
        );
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(TOKEN_BUDGET_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(TOKEN_BUDGET_ENV);
            }
        }
    }

    /// `token_budget_from_env`:malformed env → 回退 toml(不报错)。
    #[test]
    fn token_budget_from_env_malformed_falls_through_to_toml() {
        let _g = env_lock().lock().unwrap();
        let prior = std::env::var(TOKEN_BUDGET_ENV).ok();
        unsafe {
            std::env::set_var(TOKEN_BUDGET_ENV, "not-a-number");
        }
        assert_eq!(token_budget_from_env(Some(7_777)), Some(7_777));
        if let Some(p) = prior {
            unsafe {
                std::env::set_var(TOKEN_BUDGET_ENV, p);
            }
        } else {
            unsafe {
                std::env::remove_var(TOKEN_BUDGET_ENV);
            }
        }
    }

    /// `AgentConfig::with_token_budget` 设置后 `current_token_budget` 读到。
    #[test]
    fn agent_config_with_token_budget_roundtrips() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp").with_token_budget(Some(42));
        assert_eq!(cfg.current_token_budget(), Some(42));
        let cfg2 = AgentConfig::new("anthropic/x", "/tmp").with_token_budget(None);
        assert_eq!(cfg2.current_token_budget(), None);
    }

    /// `set_token_budget` 热重载路径,Clone 后副本共享同一把 RwLock。
    #[test]
    fn agent_config_set_token_budget_shared_via_clone() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        let cfg2 = cfg.clone();
        cfg.set_token_budget(Some(100));
        assert_eq!(
            cfg2.current_token_budget(),
            Some(100),
            "clone 必须能观察到 token_budget 写入"
        );
    }

    /// `session_budget_exceeded`:无预算永不超过;有预算按总量判定。
    #[test]
    fn agent_config_session_budget_exceeded_logic() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        // 无预算:即使累计很大也不过。
        cfg.add_session_usage(&reflect_protocol::TokenUsage {
            total_tokens: 1_000_000,
            ..Default::default()
        });
        assert!(!cfg.session_budget_exceeded());
        // 设预算 500,累计 1_000_000 → 超过。
        cfg.set_token_budget(Some(500));
        assert!(cfg.session_budget_exceeded());
    }

    /// `add_session_usage`:累加 input/output/total;saturating 防溢出。
    #[test]
    fn agent_config_add_session_usage_accumulates() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        cfg.add_session_usage(&reflect_protocol::TokenUsage {
            input_tokens: 100,
            output_tokens: 20,
            total_tokens: 120,
            ..Default::default()
        });
        cfg.add_session_usage(&reflect_protocol::TokenUsage {
            input_tokens: 50,
            output_tokens: 10,
            total_tokens: 60,
            ..Default::default()
        });
        let u = cfg.current_session_usage();
        assert_eq!(u.input_tokens, 150);
        assert_eq!(u.output_tokens, 30);
        assert_eq!(u.total_tokens, 180);
    }

    /// `shared_tool_context` 把 session_usage / token_budget /
    /// context_window_size 句柄共享给 `ToolContext`(get_context_remaining 工具读)。
    #[test]
    fn agent_config_shared_tool_context_shares_usage_handles() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp").with_token_budget(Some(1_000));
        *cfg.context_window_size.write() = Some(200_000);
        let ctx = cfg.shared_tool_context();
        // 写 cfg 的 session_usage,ctx 的句柄能读到(同一把锁)。
        cfg.add_session_usage(&reflect_protocol::TokenUsage {
            total_tokens: 77,
            ..Default::default()
        });
        assert_eq!(ctx.session_usage.read().total_tokens, 77);
        assert_eq!(*ctx.token_budget.read(), Some(1_000));
        assert_eq!(*ctx.context_window_size.read(), Some(200_000));
    }

    // ── v1.2 P1-12(已有-B): force_compact 标志 ───────────────────────

    /// 默认不强制压缩。
    #[test]
    fn force_compact_defaults_false() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        assert!(!cfg.take_force_compact());
    }

    /// `request_force_compact` 置 true,`take_force_compact` 读到并清零。
    #[test]
    fn force_compact_request_then_take_clears() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        cfg.request_force_compact();
        assert!(cfg.take_force_compact(), "should read true after request");
        assert!(
            !cfg.take_force_compact(),
            "take should clear the flag (second read is false)"
        );
    }

    /// Clone 后两副本共享同一把 RwLock(`/compact` 在 submission_loop 写,
    /// pre_loop 在 NodeContext 读)。
    #[test]
    fn force_compact_shared_via_clone() {
        let cfg = AgentConfig::new("anthropic/x", "/tmp");
        let cfg2 = cfg.clone();
        cfg.request_force_compact();
        assert!(
            cfg2.take_force_compact(),
            "clone must observe the force_compact write"
        );
    }
}
