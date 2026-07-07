//! `HookEvent` — the 5 events the engine dispatches.
//!
//! See `docs/tools-and-hooks.md §4.1`. Note: `HookEvent` does NOT carry
//! `ToolContext` (which lives in `reflect-tools`) directly to avoid a
//! dependency cycle. Instead, hooks that need context inspect the slim
//! fields surfaced here (workspace, session_id, turn_id, permission_mode).
//! The full `ToolContext` is available to tool implementations via
//! `Tool::execute`.

use serde::{Deserialize, Serialize};

use reflect_protocol::{PermissionMode, ThreadId, ToolError, ToolOutput, TurnId};

/// Tag for the kind of `HookEvent` — used by `Hook::events` for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEventKind {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Stop,
    SessionStart,
    // ── v1.1.0: task lifecycle events (reflect-task) ──
    /// `TaskCreate` 工具成功落盘后触发。
    TaskCreated,
    /// `TaskUpdate` 把 status 改为 `completed` 时触发(已存在任务)。
    TaskCompleted,
    /// `TaskUpdate` 改了除 status→completed 之外的字段时触发。
    TaskUpdated,
}

/// Why a stop is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model emitted a final message (no tool calls).
    AgentDecision,
    /// The graph hit its `max_iterations` safety valve.
    MaxIterations,
    /// The user interrupted (Ctrl-C).
    UserInterrupt,
}

/// Slim view of `ToolContext` exposed to hooks (avoids a tools↔hooks cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub session_id: ThreadId,
    pub turn_id: TurnId,
    pub workspace: std::path::PathBuf,
    pub permission_mode: PermissionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HookEvent {
    /// Fired before a tool runs. Hooks may `Deny`, `ModifyArgs`, or
    /// `PermissionOverride`.
    PreToolUse {
        tool: String,
        args: serde_json::Value,
        ctx: HookContext,
    },
    /// Fired after a tool returns successfully.
    PostToolUse {
        tool: String,
        result: ToolOutput,
        elapsed_ms: u64,
    },
    /// Fired after a tool returns an error (or timed out).
    PostToolUseFailure {
        tool: String,
        error: ToolError,
        elapsed_ms: u64,
    },
    /// Fired when the agent wants to finish a turn. Hooks may `Deny` to
    /// force continuation.
    Stop { reason: StopReason, attempt: u32 },
    /// Fired once at session start. Hooks receive the resolved config.
    SessionStart {
        session_id: ThreadId,
        config: serde_json::Value,
    },
    // ── v1.1.0: task lifecycle events (reflect-task) ──
    /// `TaskCreate` 工具成功落盘后触发。`task` 是任务的 JSON 快照
    ///(用 `serde_json::Value` 避免 `hooks ↔ task` 反向依赖)。
    TaskCreated { task: serde_json::Value },
    /// `TaskUpdate` 把 status 改为 `completed` 时触发。`previous_status`
    /// 是变更前的状态字符串(`"pending"` / `"in_progress"` / `"deleted"`)。
    TaskCompleted {
        task: serde_json::Value,
        previous_status: String,
    },
    /// `TaskUpdate` 改了除 status→completed 之外的字段时触发。
    TaskUpdated {
        task: serde_json::Value,
        changed_fields: Vec<String>,
    },
}

impl HookEvent {
    /// Which event kind this is (for `Hook::events` filtering).
    pub fn kind(&self) -> HookEventKind {
        match self {
            HookEvent::PreToolUse { .. } => HookEventKind::PreToolUse,
            HookEvent::PostToolUse { .. } => HookEventKind::PostToolUse,
            HookEvent::PostToolUseFailure { .. } => HookEventKind::PostToolUseFailure,
            HookEvent::Stop { .. } => HookEventKind::Stop,
            HookEvent::SessionStart { .. } => HookEventKind::SessionStart,
            HookEvent::TaskCreated { .. } => HookEventKind::TaskCreated,
            HookEvent::TaskCompleted { .. } => HookEventKind::TaskCompleted,
            HookEvent::TaskUpdated { .. } => HookEventKind::TaskUpdated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn kind_matches_variant() {
        let e = HookEvent::Stop {
            reason: StopReason::AgentDecision,
            attempt: 0,
        };
        assert_eq!(e.kind(), HookEventKind::Stop);
    }

    #[test]
    fn context_serde_roundtrip() {
        let c = HookContext {
            session_id: ThreadId::new(),
            turn_id: TurnId::new(),
            workspace: PathBuf::from("/tmp"),
            permission_mode: PermissionMode::Auto,
        };
        let back: HookContext = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.workspace, PathBuf::from("/tmp"));
    }
}
