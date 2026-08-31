//! `reflect_app_core::state` —— UI-agnostic RenderState 共享给 TUI 与 GUI。
#![allow(missing_docs)]
//!
//! **B2-01 对齐**: 从 TUI `app.rs::RenderState`（约 80 字段）抽出全部
//! UI-agnostic 状态；`Default` 实现；无 IO；不引用 ratatui/crossterm/tauri。
//!
//! 设计原则：
//! - 不引用 ratatui / crossterm / tauri（纯数据）
//! - 所有 `id` 用 `String`（transparent UUID，前端可序列化）
//! - 派生 `Clone` + `Debug` 便于测试
//! - 公开字段 + 构造器，无 builder 库
//!
//! 该 `RenderState` 与 `src/stores/agentStore.ts` 的 `AgentState` 字段
//! 一一对应（B2-08：TUI/GUI 共用同一 reducer）；TUI 端的同名字段从
//! 这里 `pub use` 出去以保持 ABI。

use std::path::PathBuf;
use std::time::SystemTime;

use reflect_protocol::{
    AbortReason, AskUserQuestionEvent, PermissionMode, PlanId, RiskLevel, RolloutRecord,
    RoutingEventKind, ThreadId, TurnId,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// 顶层 RenderState
// ============================================================================

/// 与 UI 无关的渲染状态，是 TUI 和 GUI reducer 的唯一事实来源。
/// 其结构与 `agentStore.AgentState` 相同，但去除了
/// 绑定 store 的 action 函数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderState {
    // ── 会话 / thread ─────────────────────────────────────────────
    /// 活动 thread id（首次 `SessionConfigured` 前为 `None`）。
    pub active_thread: Option<ThreadId>,

    /// 活动模型 spec（例如 `"anthropic/claude-opus-4-7"`）。
    pub active_model: String,

    /// 活动 provider 名称（`"anthropic"` / `"openai"` / …）。
    pub active_provider: String,

    /// 来自 `SessionConfigured` 的批准与 sandbox 策略。
    pub approval_policy: ApprovalPolicy,
    pub sandbox_policy: SandboxPolicy,
    pub permission_mode: PermissionMode,

    /// 上下文窗口大小（token）。未知时为 `None`。
    pub context_window_size: Option<u32>,

    // ── 推理 ─────────────────────────────────────────────────────
    pub reasoning_effort: ReasoningEffort,
    pub plan_mode: bool,

    // ── Turn 历史 ─────────────────────────────────────────────────
    pub turns: Vec<Turn>,

    // ── 待处理状态 ────────────────────────────────────────────────
    pub pending_approval: Option<PendingApproval>,
    pub pending_question: Option<PendingQuestion>,
    pub pending_ask_user: Option<PendingAskUser>,
    pub pending_plan: Option<PendingPlan>,

    // ── 会话级标志 ──────────────────────────────────────────
    pub busy: bool,
    pub last_error: Option<String>,
    pub shutdown_requested: bool,

    // ── 实时流式输出 ───────────────────────────────────────────────
    /// 当前流式 agent 消息（多 delta 累加器）。
    pub live_agent_message: String,
    pub live_thinking: String,
    pub streaming_turn: Option<TurnId>,

    // ── 批准历史（B7-05） ─────────────────────────────────────
    pub approval_history: Vec<ApprovalRecord>,

    // ── Token 统计（B1-04 token_count） ─────────────────────────
    pub last_token_usage: Option<TokenUsageSnapshot>,
    pub total_cost_usd: f64,

    // ── 路由（B1-04 routing event） ────────────────────────────────
    pub last_routing: Option<RoutingSnapshot>,

    // ── 协作（B1-04 collab_* event） ───────────────────────────────
    pub collab_sessions: Vec<CollabSession>,

    // ── MCP / LSP server 状态（B1-04 mcp/lsp_* event） ─────────────
    pub mcp_servers: Vec<ServerState>,
    pub lsp_servers: Vec<ServerState>,
    pub mcp_invocations: Vec<McpInvocation>,

    // ── 配置（B1-04 config_reloaded） ───────────────────────────────
    pub config_reloaded_at: Option<SystemTime>,

    // ── 与 UI 无关的视图状态（B2-01 扩展） ────────────────────
    /// 侧栏可见性（B12-02）。
    pub sidebar_visible: bool,
    /// Inspector 可见性。
    pub inspector_visible: bool,
    /// Statusline 模板。
    pub statusline_template: String,
    /// 活动主题调色板 id（B10-01）。
    pub active_theme: String,
    /// Keymap 覆盖（B10-03）：action 名称 → 按键组合。
    pub keymap: std::collections::BTreeMap<String, String>,
    /// 当前 command palette 查询（B10-06）。
    pub command_palette_query: String,
    pub command_palette_open: bool,
    /// 底部 terminal 面板（B8-04）是否打开。
    pub terminal_panel_open: bool,
    /// 待处理的 slash command 输入（B4）。
    pub slash_query: String,
    /// Workspace cwd（B9-06）。
    pub workspace: PathBuf,
    /// Vim 模式开关（自 v1.0 起与 UI 无关）。
    pub vim_mode: bool,
    /// 最近一次错误 toast 消息及时间戳。
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
            statusline_template:
                "{{model}} · {{tokens}}/{{contextWindow}} ({{percent}}%) · {{cwd}}".to_string(),
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
// 子类型
// ============================================================================

/// 批准策略：Auto（自动批准）/ Prompt（提示用户）/ Deny（拒绝）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    #[default]
    Auto,
    Prompt,
    Deny,
}

/// 沙箱策略：WorkspaceOnly（仅工作区）/ OsSandbox（系统沙箱）/ FullAccess（完全访问）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxPolicy {
    #[default]
    WorkspaceOnly,
    OsSandbox,
    FullAccess,
}

/// 推理强度等级：Low（低）/ Medium（中）/ High（高）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    #[default]
    Low,
    Medium,
    High,
}

/// 单次对话轮次的完整记录。
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

/// Turn 运行状态：Streaming（流式中）/ Done（完成）/ Aborted（中止）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Streaming,
    Done,
    Aborted,
}

/// 工具调用记录（含调用 ID、工具名、参数、状态）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub status: ToolStatus,
}

/// 工具调用状态：Running（执行中）/ Done（完成）/ Error（错误）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

/// 工具调用输出（含文本结果与错误标记）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub call_id: String,
    pub text: String,
    pub is_error: bool,
}

/// 待处理的审批请求（工具 / Hook / Plan）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: String,
    pub kind: ApprovalKind,
    pub tool_name: Option<String>,
    pub args_summary: Option<String>,
    /// permission-bubble 批准的可选风险等级。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<RiskLevel>,
    pub turn_id: TurnId,
}

/// 审批类型：Tool（工具）/ Hook（钩子）/ Plan（计划）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    Tool,
    Hook,
    Plan,
}

/// 待处理的用户选择题（由 `AskUserQuestionEvent` 驱动）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuestion {
    pub id: String,
    pub event: AskUserQuestionEvent,
    pub turn_id: TurnId,
}

/// 待处理的自由格式用户输入请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAskUser {
    pub id: String,
    pub prompt: String,
    pub turn_id: TurnId,
}

/// 待处理的 Plan 请求（含任务描述与可选的 Markdown 计划文本）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPlan {
    pub id: PlanId,
    pub task: String,
    pub markdown: Option<String>,
    pub turn_id: TurnId,
}

/// 审批历史记录条目（含 ID、类型、决定、时间）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub kind: ApprovalKind,
    pub tool_name: Option<String>,
    pub decision: Decision,
    pub at: SystemTime,
}

/// 审批决定：Approve（批准本次）/ Deny（拒绝）/ ApproveForSession（批准本次会话）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    Approve,
    Deny,
    ApproveForSession,
}

/// Token 用量快照（单次 turn 的输入/输出/缓存/总计/费用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    pub input: u32,
    pub output: u32,
    pub cached: u32,
    pub total: u32,
    pub cost_usd: Option<f64>,
}

/// 路由切换快照（记录请求从哪个凭证路由到了哪个凭证）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    pub kind: RoutingEventKind,
    pub role: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub reason: String,
}

/// 协作会话记录（含参与者、模式、状态、消息列表）。
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

/// 协作会话状态：Running（进行中）/ Done（完成）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollabStatus {
    Running,
    Done,
}

/// 协作会话中的单条消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabMessage {
    pub from: String,
    pub kind: String,
    pub content: String,
    pub round: u32,
    pub at: SystemTime,
}

/// MCP / LSP server 状态记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub name: String,
    pub status: ServerStatus,
    pub detail: Option<String>,
}

/// Server 运行状态：Started（已启动）/ Failed（启动失败）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    Started,
    Failed,
}

/// MCP 工具调用记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInvocation {
    pub server: String,
    pub tool: String,
    pub call_id: String,
    pub at: SystemTime,
}

/// Toast 通知消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastMessage {
    pub kind: ToastKind,
    pub text: String,
    pub at: SystemTime,
}

/// Toast 类型：Info（信息）/ Success（成功）/ Warning（警告）/ ErrorToast（错误）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    ErrorToast,
}

/// 默认 keymap——与 TUI/GUI 暴露的 action 一致。
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
// 为使用方重新导出（TUI / GUI / future WASM/NAPI）
// ============================================================================

/// 重新导出所依赖的 protocol 类型。
pub use reflect_protocol::{Event, EventMsg as ProtocolEventMsg, Submission};

/// rollout 中存储的单条记录（每个 submission 或 event 一条）。
pub type RolloutEntry = RolloutRecord;

#[allow(dead_code)]
fn _force_use(_: AbortReason) {} // 保留 AbortReason 导入，供未来使用

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
        assert!(s.sidebar_visible); // 默认值 = 可见
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
