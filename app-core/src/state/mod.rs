//! `reflect_app_core::state` —— UI-agnostic RenderState 共享给 TUI 与 GUI。
#![allow(missing_docs)]
//!
//! **B2-01 alignment**: 从 TUI `app.rs::RenderState` (~80 字段) 抽出对齐
//! zcode/Codex Desktop 所需的全部 UI-agnostic 状态;`Default` 实现;无 IO;
//! 不引用 ratatui/crossterm/tauri。
//!
//! 设计原则:
//! - 不引用 ratatui / crossterm / tauri(纯数据)
//! - 所有 `id` 用 `String`(transparent UUID,前端可序列化)
//! - 派生 `Clone` + `Debug` 便于测试
//! - 公开字段 + 构造器,无 builder 库
//!
//! 该 `RenderState` 与 `src/stores/agentStore.ts` 的 `AgentState` 字段
//! 一一对应(B2-08: TUI/GUI 共用同一 reducer);TUI 端的同名字段从
//! 这里 `pub use` 出去以保持 ABI。

use std::path::PathBuf;
use std::time::SystemTime;

use reflect_protocol::{
    AbortReason, AskUserQuestionEvent, EventMsg, PermissionMode, PlanId, RolloutRecord,
    RoutingEventKind, ThreadId, TurnId,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Top-level RenderState
// ============================================================================

/// UI-agnostic render state. Single source of truth for both TUI and GUI
/// reducers. The shape is the same as `agentStore.AgentState` minus the
/// store-bound action functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderState {
    // ── Session / thread ─────────────────────────────────────────────
    /// Active thread id (`None` until first `SessionConfigured`).
    pub active_thread: Option<ThreadId>,

    /// Active model spec (e.g. `"anthropic/claude-opus-4-7"`).
    pub active_model: String,

    /// Active provider name (`"anthropic"` / `"openai"` / …).
    pub active_provider: String,

    /// Approval + sandbox policy from `SessionConfigured`.
    pub approval_policy: ApprovalPolicy,
    pub sandbox_policy: SandboxPolicy,
    pub permission_mode: PermissionMode,

    /// Context window size (tokens). `None` if unknown.
    pub context_window_size: Option<u32>,

    // ── Reasoning ─────────────────────────────────────────────────────
    pub reasoning_effort: ReasoningEffort,
    pub plan_mode: bool,

    // ── Turn history ─────────────────────────────────────────────────
    pub turns: Vec<Turn>,

    // ── Pending state ────────────────────────────────────────────────
    pub pending_approval: Option<PendingApproval>,
    pub pending_question: Option<PendingQuestion>,
    pub pending_ask_user: Option<PendingAskUser>,
    pub pending_plan: Option<PendingPlan>,

    // ── Session-level flags ──────────────────────────────────────────
    pub busy: bool,
    pub last_error: Option<String>,
    pub shutdown_requested: bool,

    // ── Live streaming ───────────────────────────────────────────────
    /// Current streaming agent message (multi-delta accumulator).
    pub live_agent_message: String,
    pub live_thinking: String,
    pub streaming_turn: Option<TurnId>,

    // ── Approval history (B7-05) ─────────────────────────────────────
    pub approval_history: Vec<ApprovalRecord>,

    // ── Token accounting (B1-04 token_count) ─────────────────────────
    pub last_token_usage: Option<TokenUsageSnapshot>,
    pub total_cost_usd: f64,

    // ── Routing (B1-04 routing event) ────────────────────────────────
    pub last_routing: Option<RoutingSnapshot>,

    // ── Collab (B1-04 collab_* events) ───────────────────────────────
    pub collab_sessions: Vec<CollabSession>,

    // ── MCP / LSP server state (B1-04 mcp/lsp_* events) ─────────────
    pub mcp_servers: Vec<ServerState>,
    pub lsp_servers: Vec<ServerState>,
    pub mcp_invocations: Vec<McpInvocation>,

    // ── Config (B1-04 config_reloaded) ───────────────────────────────
    pub config_reloaded_at: Option<SystemTime>,

    // ── UI-agnostic view state (B2-01 extension) ────────────────────
    /// Sidebar visibility (B12-02).
    pub sidebar_visible: bool,
    /// Inspector visibility.
    pub inspector_visible: bool,
    /// Statusline template.
    pub statusline_template: String,
    /// Active theme palette id (B10-01).
    pub active_theme: String,
    /// Keymap overrides (B10-03): action name → key combination.
    pub keymap: std::collections::BTreeMap<String, String>,
    /// Active command palette query (B10-06).
    pub command_palette_query: String,
    pub command_palette_open: bool,
    /// Bottom terminal panel (B8-04) open?
    pub terminal_panel_open: bool,
    /// Pending slash command input (B4).
    pub slash_query: String,
    /// Workspace cwd (B9-06).
    pub workspace: PathBuf,
    /// Vim mode toggle (UI-agnostic since v1.0).
    pub vim_mode: bool,
    /// Last error toast message + timestamp.
    pub toast: Option<ToastMessage>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            active_thread: None,
            active_model: String::new(),
            active_provider: String::new(),
            approval_policy: ApprovalPolicy::Auto,
            sandbox_policy: SandboxPolicy::WorkspaceOnly,
            permission_mode: PermissionMode::Auto,
            context_window_size: None,
            reasoning_effort: ReasoningEffort::Low,
            plan_mode: false,
            turns: Vec::new(),
            pending_approval: None,
            pending_question: None,
            pending_ask_user: None,
            pending_plan: None,
            busy: false,
            last_error: None,
            shutdown_requested: false,
            live_agent_message: String::new(),
            live_thinking: String::new(),
            streaming_turn: None,
            approval_history: Vec::new(),
            last_token_usage: None,
            total_cost_usd: 0.0,
            last_routing: None,
            collab_sessions: Vec::new(),
            mcp_servers: Vec::new(),
            lsp_servers: Vec::new(),
            mcp_invocations: Vec::new(),
            config_reloaded_at: None,
            sidebar_visible: true,
            inspector_visible: true,
            statusline_template: "{{model}} · {{tokens}}/{{contextWindow}} ({{percent}}%) · {{cwd}}".to_string(),
            active_theme: "dark-default".to_string(),
            keymap: default_keymap(),
            command_palette_query: String::new(),
            command_palette_open: false,
            terminal_panel_open: false,
            slash_query: String::new(),
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            vim_mode: false,
            toast: None,
        }
    }
}

// ============================================================================
// Sub-types
// ============================================================================

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    #[default]
    Auto,
    Prompt,
    Deny,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxPolicy {
    #[default]
    WorkspaceOnly,
    OsSandbox,
    FullAccess,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub id: TurnId,
    pub user_text: Option<String>,
    pub assistant_text: String,
    pub thinking: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_outputs: Vec<ToolOutput>,
    pub error: Option<String>,
    pub status: TurnStatus,
    pub started_at: SystemTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Streaming,
    Done,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub status: ToolStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub call_id: String,
    pub text: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: String,
    pub kind: ApprovalKind,
    pub tool_name: Option<String>,
    pub args_summary: Option<String>,
    pub turn_id: TurnId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    Tool,
    Hook,
    Plan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuestion {
    pub id: String,
    pub event: AskUserQuestionEvent,
    pub turn_id: TurnId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAskUser {
    pub id: String,
    pub prompt: String,
    pub turn_id: TurnId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPlan {
    pub id: PlanId,
    pub task: String,
    pub markdown: Option<String>,
    pub turn_id: TurnId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub kind: ApprovalKind,
    pub tool_name: Option<String>,
    pub decision: Decision,
    pub at: SystemTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    Approve,
    Deny,
    ApproveForSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    pub input: u32,
    pub output: u32,
    pub cached: u32,
    pub total: u32,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    pub kind: RoutingEventKind,
    pub role: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabSession {
    pub id: String,
    pub participants: Vec<String>,
    pub mode: String,
    pub status: CollabStatus,
    pub outcome: Option<String>,
    pub rounds: Option<u32>,
    pub messages: Vec<CollabMessage>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollabStatus {
    Running,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabMessage {
    pub from: String,
    pub kind: String,
    pub content: String,
    pub round: u32,
    pub at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub name: String,
    pub status: ServerStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    Started,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInvocation {
    pub server: String,
    pub tool: String,
    pub call_id: String,
    pub at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastMessage {
    pub kind: ToastKind,
    pub text: String,
    pub at: SystemTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    ErrorToast,
}

/// Default keymap — matches the actions the TUI/GUI expose.
pub fn default_keymap() -> std::collections::BTreeMap<String, String> {
    let mut m = std::collections::BTreeMap::new();
    m.insert("submit".into(), "Cmd+Enter".into());
    m.insert("focus_composer".into(), "Cmd+/".into());
    m.insert("command_palette".into(), "Cmd+K".into());
    m.insert("slash_palette".into(), "Cmd+Shift+P".into());
    m.insert("close_modal".into(), "Escape".into());
    m.insert("interrupt".into(), "Cmd+I".into());
    m.insert("toggle_sidebar".into(), "Cmd+B".into());
    m.insert("toggle_inspector".into(), "Cmd+J".into());
    m.insert("toggle_terminal".into(), "Cmd+`".into());
    m.insert("cycle_permission".into(), "Shift+Tab".into());
    m
}

// ============================================================================
// Re-export for consumers (TUI / GUI / future WASM/NAPI)
// ============================================================================

/// Re-export of the protocol types we depend on.
pub use reflect_protocol::{Event, EventMsg as ProtocolEventMsg, Submission};

/// Single record stored in the rollout (one per submission or event).
pub type RolloutEntry = RolloutRecord;

#[allow(dead_code)]
fn _force_use(_: AbortReason) {} // keep AbortReason import live for future use

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_render_state_is_empty() {
        let s = RenderState::default();
        assert!(s.active_thread.is_none());
        assert_eq!(s.permission_mode, PermissionMode::Auto);
        assert_eq!(s.reasoning_effort, ReasoningEffort::Low);
        assert!(!s.busy);
        assert!(s.turns.is_empty());
        assert!(s.live_agent_message.is_empty());
        assert!(s.sidebar_visible); // default = visible
    }

    #[test]
    fn default_keymap_has_core_shortcuts() {
        let km = default_keymap();
        assert_eq!(km.get("submit").map(|s| s.as_str()), Some("Cmd+Enter"));
        assert!(km.contains_key("command_palette"));
    }

    #[test]
    fn render_state_serde_roundtrip() {
        let s = RenderState::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: RenderState = serde_json::from_str(&j).unwrap();
        assert_eq!(back.permission_mode, s.permission_mode);
    }

    #[test]
    fn approval_kind_serde_uses_snake_case() {
        let k = ApprovalKind::Tool;
        let j = serde_json::to_string(&k).unwrap();
        assert_eq!(j, "\"tool\"");
    }

    #[test]
    fn turn_status_serde_uses_snake_case() {
        let j = serde_json::to_string(&TurnStatus::Streaming).unwrap();
        assert_eq!(j, "\"streaming\"");
    }
}
