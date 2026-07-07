//! Tool trait and its companion types.
//!
//! `ToolError` and `ToolOutput` live in `reflect-protocol` to break the
//! `reflect-tools` ↔ `reflect-hooks` dependency cycle. The `Tool`
//! trait and built-in tools remain here.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;
use tokio_util::sync::CancellationToken;

use reflect_protocol::{PermissionMode, ThreadId, TurnId};

use crate::approval::ApprovalGate;
use crate::worktree::SessionWorktreeState;

pub use reflect_protocol::ToolError;
pub use reflect_protocol::ToolOutput;

/// A capability the model can invoke. Stateless (state lives in `ToolRegistry`).
#[async_trait]
pub trait Tool: Send + Sync {
    /// Stable tool name (e.g. `"bash"`).
    fn name(&self) -> &str;

    /// Human/LLM-readable description; sent to the model.
    fn description(&self) -> &str;

    /// JSON Schema describing accepted arguments.
    fn parameters_schema(&self) -> serde_json::Value;

    /// True if calls to this tool can safely run concurrently with each other
    /// (and with other concurrency-safe tools). Read-only tools return `true`.
    /// Side-effecting tools (file mutation, network, exec) return `false`.
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    /// Permission required to invoke this tool. Defaults to `Auto` — the
    /// tool runs without prompting. Tools that mutate state (`bash`, `write`,
    /// `edit`) should override to `Prompt` so the `ToolExecutionQueue`
    /// triggers the M6 approval flow via `ApprovalGate`.
    fn required_permission(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    /// Per-action 权限路由(v1.0.0-rc1+):多 action 单 tool(如 `ast`
    /// / `lsp`)用这个方法按具体 action 决定 `Auto` vs `Prompt`,避免
    /// 整体 tool 走 Prompt 阻塞 LLM 试探(`lsp` 的 read action 应该 Auto,
    /// `ast` 的 `search` 应该 Auto,但 `ast` 的 `replace` 应该 Prompt)。
    ///
    /// 默认 fallback 到 [`required_permission`](Self::required_permission)
    /// —— 单 action 工具不需要覆盖这个方法。
    fn action_permission(&self, _args: &serde_json::Value) -> PermissionMode {
        self.required_permission()
    }

    /// Execute the tool. The queue fills in `elapsed_ms` on the returned
    /// `ToolOutput` (M2+); M1 tools return 0.
    async fn execute(
        &self,
        ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolOutput, ToolError>;
}

/// Context passed to every tool invocation.
///
/// `ToolContext` does NOT carry the `HookEngine` — the engine is held by
/// the `ToolExecutionQueue` itself. Tools that need to know about hook
/// state inspect `ToolContext::metadata` (where `InjectMessage` reminders
/// are stashed by the queue before calling `execute`).
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// 工作区根路径 —— 与 `AgentConfig.workspace` 共享同一把 `RwLock`,
    /// `EnterWorktreeTool` 可在会话内热切换。
    pub workspace: Arc<RwLock<PathBuf>>,
    /// 当前 worktree 会话状态;`None` 表示在主 workspace。
    pub worktree: Arc<RwLock<Option<SessionWorktreeState>>>,
    pub cancel: CancellationToken,
    pub timeout: Duration,
    pub call_id: String,
    /// Stable session id (M3+).
    pub session_id: ThreadId,
    /// Per-turn id (M3+).
    pub turn_id: TurnId,
    /// Environment variables to inject (M3+; used by subagents).
    pub env: HashMap<String, String>,
    /// Effective permission mode for this call (M3+; mutated by
    /// `HookDecision::PermissionOverride`).
    pub permission_mode: PermissionMode,
    /// Per-call scratch metadata (M3+; stashes hook-injected reminders
    /// under the `system_reminder` key).
    pub metadata: serde_json::Value,
    /// v1.0.0-rc1+:可选的 approval gate 句柄,由 `ToolExecutionQueue`
    /// 在 `execute_single` 入口注入 `Some(Arc<ApprovalGate>)`,工具内
    /// 可以走 `gate.ask_tool(name, args, ...)` 主动触发 prompt-permission
    /// modal(典型场景:per-action 权限决策 —— 同 tool 不同 action 走不同
    /// approval 路径)。`None` 意味着 headless 模式(无 TUI),不主动 prompt。
    pub approval: Option<Arc<ApprovalGate>>,
    /// v1.2 P1-12:会话级累计 token 用量(跨 turn 累加)。与 `AgentConfig`
    /// / `NodeContext` 共享同一把 `Arc<RwLock<TokenUsage>>`。`model_call`
    /// 每次调用后累加;`get_context_remaining` 工具读它算剩余预算。
    /// `Arc::new(RwLock::new(TokenUsage::default()))` 默认值让无引擎上下文
    /// 的测试 / 单次调用也能工作(读到全零)。
    pub session_usage: Arc<RwLock<reflect_protocol::TokenUsage>>,
    /// v1.2 P1-12:会话 token 预算硬上限(共享句柄)。`get_context_remaining`
    /// 用它报告预算剩余。`None` = 无预算上限。
    pub token_budget: Arc<RwLock<Option<u64>>>,
    /// v1.2 P1-12:当前 model 的上下文窗口大小(token),引擎在
    /// `SessionConfigured` 时用 `reflect_llm::context_window_for` 算出后
    /// 写入(共享句柄,热重载切 model 后刷新)。`get_context_remaining`
    /// 用它做「已用 / 总量」的分母。`None` = 未知 model。
    pub context_window_size: Arc<RwLock<Option<u32>>>,
}

impl ToolContext {
    /// 从路径构造 `ToolContext`(测试 / 单次调用用)。
    pub fn for_workspace(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: Arc::new(RwLock::new(workspace.into())),
            ..Default::default()
        }
    }

    /// 与 `AgentConfig` 共享 workspace / worktree 句柄(bootstrap 用)。
    pub fn with_shared(
        workspace: Arc<RwLock<PathBuf>>,
        worktree: Arc<RwLock<Option<SessionWorktreeState>>>,
    ) -> Self {
        Self {
            workspace,
            worktree,
            ..Default::default()
        }
    }

    /// 当前 workspace 快照。
    pub fn workspace_path(&self) -> PathBuf {
        self.workspace.read().clone()
    }

    /// 热切换 workspace(与 `AgentConfig::set_workspace` 写同一把锁)。
    pub fn set_workspace(&self, path: PathBuf) {
        *self.workspace.write() = path;
    }

    /// 读取 worktree 会话状态快照。
    pub fn worktree_state(&self) -> Option<SessionWorktreeState> {
        self.worktree.read().clone()
    }

    /// 写入 worktree 会话状态。
    pub fn set_worktree_state(&self, state: Option<SessionWorktreeState>) {
        *self.worktree.write() = state;
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            workspace: Arc::new(RwLock::new(PathBuf::new())),
            worktree: Arc::new(RwLock::new(None)),
            cancel: CancellationToken::new(),
            timeout: Duration::from_secs(30),
            call_id: String::new(),
            session_id: ThreadId::new(),
            turn_id: TurnId::new(),
            env: HashMap::new(),
            permission_mode: PermissionMode::Auto,
            metadata: serde_json::json!({}),
            approval: None,
            session_usage: Arc::new(RwLock::new(reflect_protocol::TokenUsage::default())),
            token_budget: Arc::new(RwLock::new(None)),
            context_window_size: Arc::new(RwLock::new(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct Dummy;
    #[async_trait]
    impl Tool for Dummy {
        fn name(&self) -> &str {
            "dummy"
        }
        fn description(&self) -> &str {
            "no-op"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        fn is_concurrency_safe(&self) -> bool {
            true
        }
        async fn execute(
            &self,
            _: ToolContext,
            _: serde_json::Value,
        ) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                content: vec![],
                is_error: false,
                metadata: serde_json::Value::Null,
                elapsed_ms: 0,
            })
        }
    }

    #[test]
    fn default_ctx_has_thirty_second_timeout() {
        let c = ToolContext::default();
        assert_eq!(c.timeout, Duration::from_secs(30));
    }

    #[test]
    fn dummy_says_concurrency_safe() {
        assert!(Dummy.is_concurrency_safe());
    }

    #[test]
    fn default_ctx_starts_in_auto_mode() {
        let c = ToolContext::default();
        assert_eq!(c.permission_mode, PermissionMode::Auto);
    }
}
