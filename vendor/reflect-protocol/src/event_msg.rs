//! EventMsg — the discriminated union of all events the core can emit.
//!
//! v0 has 17 variants. New variants are non-breaking additions (additive serde).
//! Skipped for M1/M2: `ToolCallOutputDelta` (deferred to v1).
//! M6 added: `ApprovalRequest` (paired with `Op::ToolApproval` / `Op::HookApproval`).
//! M7 added: `ConfigReloaded`.
//! M10/v0.2.4 added: `CollabStarted` / `CollabMessage` / `CollabFinished` so the
//! discussion feature becomes visible to TUI / headless consumers.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

pub use crate::ask_user_input::AskUserInputEvent;
use crate::item::{
    ApprovalPolicy, PermissionMode, PlanId, RiskLevel, SandboxPolicy, SessionConfiguredEvent,
    ThreadId, ToolOutput, TurnId,
};
pub use crate::question::AskUserQuestionEvent;

/// Stable string discriminator for logs.
pub type EventMsgDiscriminant = &'static str;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventMsg {
    // Lifecycle (5)
    /// Emitted once on first turn of a thread. The id field will be `EVENT_ID_NONE`.
    SessionConfigured(SessionConfiguredEvent),
    TurnStarted(TurnStartedEvent),
    TurnComplete(TurnCompleteEvent),
    TurnAborted(TurnAbortedEvent),
    /// 批次十九:`Op::Rewind` 成功后发出(TUI 据此裁剪显示)。
    TurnRewound(TurnRewoundEvent),
    ShutdownComplete,

    // LLM output (4)
    /// Non-streaming assistant message (rare; M1 emits deltas).
    AgentMessage(AgentMessage),
    /// Streaming assistant message chunk.
    AgentMessageDelta(AgentMessageDelta),
    /// Anthropic extended-thinking chunk.
    ThinkingDelta(ThinkingDelta),
    /// Token usage snapshot (typically emitted at end of turn).
    TokenCount(TokenCountEvent),

    // Tool (2; v1 adds ToolCallOutputDelta)
    ToolCallBegin(ToolCallBeginEvent),
    ToolCallEnd(ToolCallEndEvent),

    // Approval (1; M6)
    /// Tool or hook is awaiting user approval. The client should respond with
    /// `Op::ToolApproval { id, decision }` or `Op::HookApproval { id, decision }`
    /// matching `request_id`.
    ApprovalRequest(ApprovalRequestEvent),

    // AskUserQuestion (1; v1.1.0) — LLM 主动发起结构化询问,多题多选项。
    // 客户端通过 `Op::AskUserQuestionResponse { id, answers }` 回执。
    /// LLM 主动向用户发起 1-4 道结构化问题(每题 2-4 选项,可选
    /// multi_select,可填 "Other" 自定义文本)。TUI 收到后弹多题 modal,
    /// 用户按键 → 回执 `Op::AskUserQuestionResponse { id, answers }`,
    /// `ApprovalGate` 的对应 oneshot 收到答案后返回 `AskUserAnswer` 给
    /// `AskUserQuestionTool::execute`,最终进 LLM 消息流。
    AskUserQuestion(AskUserQuestionEvent),

    // ask_user (1; v1.1.0 P1) — LLM 主动发起自由文本询问。
    /// LLM 通过 `ask_user` 工具向用户提问(单行自由文本)。TUI 弹单行
    /// input modal;用户提交 → `Op::AskUserInputResponse { id, text }`。
    AskUserInput(AskUserInputEvent),

    /// Bubble 权限模式下的非阻塞工具执行通知(不等待用户决策)。
    PermissionBubble(PermissionBubbleEvent),

    // Compaction (1)
    ContextCompacted(ContextCompactedEvent),

    // Error (2)
    /// Recoverable error inside a turn.
    Error(ErrorEvent),
    /// Transient LLM stream error (core will retry).
    StreamError(StreamErrorEvent),

    // Config (1; M7)
    /// `~/.reflect/config.toml` was reloaded by the file watcher; provider
    /// clients in the `ModelRegistry` may have been replaced. UI may show
    /// a brief indicator.
    ConfigReloaded(ConfigReloadedEvent),

    // Routing (1; v1.0) — 多 Provider 路由的 failover / cooldown 事件
    Routing(RoutingEvent),

    // Collab (3; M10 / v0.2.4) — 讨论生命周期,TUI / headless 都可观察
    /// Discussion started. Emitted exactly once per `DiscussionOrchestrator::run`
    /// at entry, before the first `prompt_for` step. UI may show a status pill.
    CollabStarted(CollabStartedEvent),
    /// One `DiscussionMessage` was routed through the bus. Emitted after
    /// `bus.route` returns, on the same code path that calls
    /// `on_event(OrchestratorEvent::AgentTurn)`. `token_usage` is `Some` only
    /// when the spawning thread observed a `TokenCount` event via
    /// `SpawnedChild::collect_result_with_usage`; older callers emit `None`.
    CollabMessage(CollabMessageEvent),
    /// Discussion finished; mirrors the existing `OrchestratorEvent::Finished`
    /// but at the protocol layer so headless consumers (`reflect exec |
    /// jq`) and TUI can react without subscribing to `OrchestratorEvent`.
    CollabFinished(CollabFinishedEvent),

    // MCP (3; v0.3) — MCP server 启停 / 调用可见,TUI status_bar / JSONL 推送
    /// 一个 MCP server 握手 + list_tools 成功。`tool_count` 给 TUI 显示用。
    McpServerStarted(McpServerStartedEvent),
    /// 一个 MCP server 启动失败(`spawn` / `initialize` / `list_tools` 任一阶段)。
    /// `will_retry` 为 `true` 表示 HTTP 重连循环还会继续尝试。
    McpServerFailed(McpServerFailedEvent),
    /// 单次 MCP tool call 完成(透传自 LLM tool_call / reflect exec 内部),
    /// 便于 TUI 在 status_bar 高亮"mcp: server.tool"调用链。
    McpToolInvoked(McpToolInvokedEvent),

    // LSP (2; v0.5) — LSP server 启停可见,TUI / JSONL 推送
    /// 一个 LSP server 握手 + initialize 成功。`methods` 字段是从
    /// `ServerCapabilities` 推出来的 method 列表,`language_ids` 是该
    /// server 接管的 LSP languageId 集合(去重)。
    LspServerStarted(LspServerStartedEvent),
    /// 一个 LSP server 启动失败(spawn / initialize 任一阶段)。
    /// `will_retry` 永远为 `false`(LSP 不自动重连,配置错就让用户修)。
    LspServerFailed(LspServerFailedEvent),

    // Plan (5; v1.x) — Plan mode 生命周期,TUI / headless 都可观察
    /// `EnterPlanModeTool` 或 `/plan <task>` slash 触发;core 收到后
    /// 弹出 modal 让用户确认进入 Plan mode。**确认后才**把
    /// `PermissionMode` 切到 `Plan`,而不是立即切换。
    PlanRequest(PlanRequestEvent),
    /// `ExitPlanModeTool` 触发;agent 调研结束,plan markdown 已就绪,
    /// 等用户在 TUI modal 上审批。审批通过后切回 `Prompt` 模式,
    /// 写工具(bash/edit/write)解锁。
    PlanReady(PlanReadyEvent),
    /// 用户在 plan approval modal 上选择 approve。`plan_id` 与
    /// `PlanReady` 的 id 对应,便于前端配对渲染。
    PlanApproved(PlanApprovedEvent),
    /// 用户在 plan approval modal 上选择 reject。`reason` 是可选的用户
    /// 反馈(目前 slash/TUI modal 还没收集具体文本,留 `None`)。
    PlanRejected(PlanRejectedEvent),
    /// `PermissionMode` 状态机切换通知。TUI 收到后立即更新 status bar
    /// 的 `│ plan mode` 黄色 segment,所有订阅方(hook 引擎、tool queue)
    /// 都会读到新 mode。`from` / `to` 都填,便于客户端在日志中追溯。
    PermissionModeChanged(PermissionModeChangedEvent),
}

// ── Payload structs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageDelta {
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingDelta {
    pub delta: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenCountEvent {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: u32,
    /// M8: cache_creation segment (folded into `input_tokens`); zero on
    /// providers without prompt caching. `#[serde(default)]` keeps the
    /// event wire-compatible with M7 consumers.
    #[serde(default)]
    pub cache_write_tokens: u32,
    pub total_tokens: u32,
    /// M8 P1a: per-turn USD cost, computed via `reflect_llm::providers::pricing`.
    /// `None` for unknown model or when pricing table is empty. `#[serde(default)]`
    /// keeps the event wire-compatible with M7 consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// v1.0 多 Provider 路由:本轮实际命中的 provider 名(如 `"anthropic"`)。
    /// `#[serde(default, skip_serializing_if = "Option::is_none")]` 双向
    /// 兼容:旧 consumer 解析时填 `None`,序列化时缺省。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// v1.0 多 Provider 路由:命中的 credential label(如 `"work"` /
    /// `"default"`)。同 `provider` 字段的兼容策略。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStartedEvent {
    pub turn_id: TurnId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCompleteEvent {
    pub turn_id: TurnId,
    pub usage: TokenUsage,
    pub status: TurnStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnAbortedEvent {
    pub turn_id: TurnId,
    pub reason: AbortReason,
}

/// 批次十九:`Op::Rewind` 成功后发出,让 TUI 裁剪显示到回退点。
/// `truncated_after` = 被丢弃的 turn 数(0 = 已是最近一条,无可回退)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRewoundEvent {
    /// 回退到的 turn(`None` = 回退到 session 起 / 最近 user turn)。
    pub to_turn_id: Option<String>,
    /// 被丢弃的 turn 数(供 TUI Pill 文案)。
    pub truncated_after: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallBeginEvent {
    pub call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEndEvent {
    pub call_id: String,
    pub output: ToolOutput,
    pub is_error: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestEvent {
    /// Unique id; the client echoes it back in `Op::ToolApproval` / `Op::HookApproval`.
    pub request_id: String,
    /// What is being approved (tool call or hook decision).
    pub kind: ApprovalKind,
    /// Risk level (UI hint; not enforced).
    #[serde(default)]
    pub risk: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCompactedEvent {
    pub strategy: ContextCompactedStrategy,
    pub removed_messages: usize,
    pub before_tokens: u32,
    pub after_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StreamErrorEvent {
    pub code: String,
    pub message: String,
    pub retry_in_ms: u64,
    /// v1.0 多 Provider 路由:失败 credential 的 provider 名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// v1.0 多 Provider 路由:失败 credential 的 label。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_label: Option<String>,
    /// v1.0 多 Provider 路由:全部 candidate 试完仍失败时,记录每个
    /// 候选的结果。便于 TUI 渲染"5 个 key 都试过:work=429,
    /// personal=auth, ..."诊断表。`None` 表示未填(单 credential 失败
    /// 或仍在重试中)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tried: Option<Vec<TriedCredential>>,
}

/// v1.0 多 Provider 路由:单个 credential 失败时记录在 `StreamError.tried`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriedCredential {
    pub label: String,
    /// 失败原因码(同 `LlmError` 变体名):
    /// `"rate_limited"` / `"auth"` / `"overloaded"` /
    /// `"provider_5xx"` / `"network"` / `"context_too_long"` / ...
    pub outcome: String,
}

/// v1.0 多 Provider 路由:failover / cooldown 状态变化事件。TUI
/// 收到后画一行 status(`↻ main switched work → personal (rate_limited)`)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingEvent {
    pub kind: RoutingEventKind,
    /// `"main"` / `"compact"` / `"subagent:researcher"`。
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_credential: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_credential: Option<String>,
    /// 同 `TriedCredential.outcome`,描述切换原因。
    pub reason: String,
    /// 距 cooldown 到期的毫秒数(可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_until_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingEventKind {
    /// 切到了下一个可用 credential。
    Switched,
    /// 全部 candidate 失败,无路可走。
    FailedOver,
    /// 单 credential 进 cooldown 暂避。
    CooldownStarted,
    /// 成功调用后清除 cooldown(标记恢复)。
    CooldownCleared,
}

/// M7: 配置文件热重载事件。`path` 是触发变更的文件;`sections_changed` 是
/// 受影响段名列表(例如 `["anthropic", "compact"]`),便于 UI 只在关键段变更
/// 时提示。`at` 用 `SystemTime` 而非 `Instant`,因为事件可能跨进程持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigReloadedEvent {
    pub path: PathBuf,
    pub sections_changed: Vec<String>,
    #[serde(with = "systemtime_serde")]
    pub at: SystemTime,
}

/// M10/v0.2.4: 讨论启动事件。`id` 是 `DiscussionId` UUID 的字符串形式;
/// `participants` 与 `mode` 直接镜像 orchestrator 启动时的配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollabStartedEvent {
    pub id: String,
    pub participants: Vec<String>,
    pub mode: String,
}

/// M10/v0.2.4: 单条讨论消息事件。每次 `MessageBus::route` 成功投递后
/// orchestrator 会发出一次。`token_usage` 仅在 LLM 路径
///(`collect_result_with_usage` 提供 usage)非空时填入;老 caller 与
/// `run_noop` 路径发出的消息为 `None`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollabMessageEvent {
    pub id: String,
    pub from: String,
    /// 消息种类序列化形式:`"utterance"` | `"consensus"` | `"finish"`。
    /// 用字符串而非 enum 以避免 `protocol ↔ discussion` 循环依赖。
    pub kind: String,
    pub content: String,
    pub round: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
}

/// M10/v0.2.4: 讨论结束事件。`outcome` 序列化形式
/// `consensus` | `no_consensus` | `finished`;`rounds` 是实际跑过的轮数
///(可能小于 `max_rounds`)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollabFinishedEvent {
    pub id: String,
    pub outcome: String,
    pub rounds: u32,
}

// ── MCP events (v0.3) ──────────────────────────────────────────────────────

/// v0.3 M6: 一个 MCP server 握手 + `list_tools` 成功。
///
/// `tool_count` 由 manager 启动路径填,给 TUI 在 status_bar 显示
/// `│ mcp: N servers / M tools` 用。`transport` 是镜像 enum,
/// 因为 `reflect-protocol` 不能依赖 `reflect-config` (反向依赖风险),
/// `reflect-mcp::config` 提供 `From<McpTransportMirror> for McpTransport`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerStartedEvent {
    pub server: String,
    pub tool_count: usize,
    /// 批次十八:工具全名列表(供 TUI `/mcp` overlay 展示工具清单)。
    #[serde(default)]
    pub tool_names: Vec<String>,
    pub transport: McpTransportMirror,
}

/// v0.3 M6: 一个 MCP server 启动失败(spawn / initialize / list_tools 任一阶段)。
///
/// `will_retry` 为 `true` 表示 HTTP 重连循环还会继续尝试;
/// stdio 路径永远为 `false`(不重连,配置错就让用户修)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerFailedEvent {
    pub server: String,
    pub error: String,
    pub will_retry: bool,
}

/// v0.3 M6: 单次 MCP tool call 完成。
///
/// `server.tool` 拼接字符串直接给 TUI 在 status_bar 高亮调用链。
/// `call_id` 与 `EventMsg::ToolCallEnd.call_id` 一致,便于前端配对渲染。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolInvokedEvent {
    pub server: String,
    pub tool: String,
    pub call_id: String,
}

/// v0.3 M6: `reflect_config::McpTransport` 的镜像,避免 protocol → config 反向依赖。
///
/// 用 `lowercase` 序列化(`"stdio"` / `"http"`),与 config schema 端一致。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportMirror {
    Stdio,
    Http,
    /// Legacy MCP HTTP+SSE transport。
    Sse,
}

impl McpTransportMirror {
    /// 小写字符串,与 serde 序列化一致(`"stdio"` / `"http"` / `"sse"`)。
    /// TUI overlay / Pill 渲染用此避免每处都写 match。
    pub fn as_str(self) -> &'static str {
        match self {
            McpTransportMirror::Stdio => "stdio",
            McpTransportMirror::Http => "http",
            McpTransportMirror::Sse => "sse",
        }
    }
}

impl std::fmt::Display for McpTransportMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── LSP events (v0.5) ────────────────────────────────────────────────────

/// v0.5: 一个 LSP server 握手 + `initialize` 成功。
///
/// `methods` 字段是从 `ServerCapabilities` 推出来的 method 列表(当前
/// Phase A 是 `textDocument/definition` / `references` / `hover` 之一
/// 或多者),`language_ids` 是该 server 接管的 LSP languageId 集合
///(去重)。TUI status_bar 可显示 `│ lsp: N servers` + 每个 server
/// 拥有的 method 数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LspServerStartedEvent {
    pub server: String,
    pub methods: Vec<String>,
    pub language_ids: Vec<String>,
}

/// v0.5: 一个 LSP server 启动失败(`spawn` / `initialize` 任一阶段)。
///
/// `will_retry` 永远为 `false`(LSP 不自动重连,配置错就让用户修)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LspServerFailedEvent {
    pub server: String,
    pub error: String,
    pub will_retry: bool,
}

// ── Plan mode events (v1.x) ─────────────────────────────────────────────────

/// v1.x Plan mode: agent 请求进入 Plan mode。等用户审批后 core 才
/// 把 `PermissionMode` 切到 `Plan`。
///
/// `task` 是用户/agent 描述的规划目标(如 `"refactor auth module"`),
/// TUI 弹窗和 approval reason 都会展示。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanRequestEvent {
    pub task: String,
}

/// v1.x Plan mode: agent 调研结束,plan markdown 已生成。等用户审批
/// 后 core 把 `PermissionMode` 切回 `Prompt`,写工具解锁。
///
/// `markdown` 是 plan 的完整内容,通常由 agent 把最近的 tool 调研
/// 结果汇总成 markdown。`plan_id` 用于前后端配对 `PlanApproved` /
/// `PlanRejected`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanReadyEvent {
    pub plan_id: PlanId,
    pub markdown: String,
}

/// v1.x Plan mode: 用户在 approval modal 上 approve plan。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanApprovedEvent {
    pub plan_id: PlanId,
}

/// v1.x Plan mode: 用户在 approval modal 上 reject plan。`reason` 是
/// 可选的用户反馈文本(v1.x 暂未在 TUI 收集,留 `None`;后续 v1.x+1
/// 加 review comment 时填具体原因)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanRejectedEvent {
    pub plan_id: PlanId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// v1.x Plan mode: `PermissionMode` 状态机切换通知。
///
/// `from` / `to` 都填便于客户端追溯;通常 `to` 是 `Plan`(进入)
/// 或 `Prompt`(退出批准后回到 Prompt)。任何订阅方(hook 引擎、
/// tool queue、TUI status bar)都根据这个事件更新本地视图。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionModeChangedEvent {
    pub from: PermissionMode,
    pub to: PermissionMode,
}

/// Bubble 权限模式下的非阻塞工具执行通知。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionBubbleEvent {
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_preview: Option<String>,
    #[serde(default)]
    pub risk: RiskLevel,
}

// ── Sub-types ───────────────────────────────────────────────────────────────

/// Token usage snapshot. Mirrors `ChatEvent::Usage` from `reflect-llm`.
///
/// **Token accounting (M8)**: `input_tokens` is the raw `input` field the
/// LLM reports (Anthropic's `input_tokens` already folds in the
/// `cache_creation_input_tokens` subsegment). `cached_tokens` is the
/// `cache_read_input_tokens` discount subsegment — a *subset* of
/// `input_tokens`, not additive. `cache_write_tokens` is the
/// `cache_creation_input_tokens` subsegment — also a subset of
/// `input_tokens`. For billing purposes use `pricing::price(model, usage)`,
/// which apportions the four segments to their respective multipliers.
///
/// `total_tokens = input_tokens + output_tokens` reflects the on-the-wire
/// billable count and is **deliberately unchanged from M7**; do not add
/// `cache_write_tokens` or `cached_tokens` to it (that would double-count).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: u32,
    /// M8: cache_creation subsegment. Folded into `input_tokens`; do NOT
    /// add to `total_tokens`. `#[serde(default)]` keeps it wire-compatible
    /// with M7 consumers that omit the field.
    #[serde(default)]
    pub cache_write_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32, cached: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cached_tokens: cached,
            cache_write_tokens: 0,
            total_tokens: input + output,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Success,
    MaxIterations,
    Stopped,
    /// v1.2 P1-12:会话级 token 预算耗尽(`[token_budget].session_total_tokens`
    /// 或 env `REFLECT_TOKEN_BUDGET` 设的上限)后终止当前 turn。与
    /// `MaxIterations`(步数上限)区分,便于 TUI / 日志归因。
    TokenBudgetExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AbortReason {
    UserInterrupt,
    Error { code: String, message: String },
    Shutdown,
}

/// What is being approved. Tool calls carry the tool name + args; hook approvals
/// carry the hook name + a short human-readable preview of the decision payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalKind {
    Tool {
        tool_name: String,
        args: serde_json::Value,
    },
    Hook {
        hook_name: String,
        decision_preview: String,
    },
    /// v1.x Plan mode: 用户审批 plan markdown。`summary` 是 plan 的
    /// 简短预览(首 100 字符),TUI modal 用作预览文字;
    /// `plan_id` 与 `PlanReady` / `PlanApproved` / `PlanRejected` 的 id
    /// 对应,便于前端把 approval 与具体 plan 配对渲染。
    Plan { plan_id: PlanId, summary: String },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextCompactedStrategy {
    /// No-op used for M1 stub.
    #[default]
    Noop,
    /// Local heuristic (M4).
    Microcompact,
    /// Threshold-triggered (M4).
    SmartPrune,
    /// LLM-summarized (M4).
    LlMSummarize,
}

impl EventMsg {
    /// Stable string discriminator.
    pub fn discriminant(&self) -> EventMsgDiscriminant {
        match self {
            EventMsg::SessionConfigured(_) => "session_configured",
            EventMsg::TurnStarted(_) => "turn_started",
            EventMsg::TurnComplete(_) => "turn_complete",
            EventMsg::TurnAborted(_) => "turn_aborted",
            EventMsg::TurnRewound(_) => "turn_rewound",
            EventMsg::ShutdownComplete => "shutdown_complete",
            EventMsg::AgentMessage(_) => "agent_message",
            EventMsg::AgentMessageDelta(_) => "agent_message_delta",
            EventMsg::ThinkingDelta(_) => "thinking_delta",
            EventMsg::TokenCount(_) => "token_count",
            EventMsg::ToolCallBegin(_) => "tool_call_begin",
            EventMsg::ToolCallEnd(_) => "tool_call_end",
            EventMsg::ApprovalRequest(_) => "approval_request",
            EventMsg::AskUserQuestion(_) => "ask_user_question",
            EventMsg::AskUserInput(_) => "ask_user",
            EventMsg::PermissionBubble(_) => "permission_bubble",
            EventMsg::ContextCompacted(_) => "context_compacted",
            EventMsg::Error(_) => "error",
            EventMsg::StreamError(_) => "stream_error",
            EventMsg::ConfigReloaded(_) => "config_reloaded",
            EventMsg::CollabStarted(_) => "collab_started",
            EventMsg::CollabMessage(_) => "collab_message",
            EventMsg::CollabFinished(_) => "collab_finished",
            EventMsg::McpServerStarted(_) => "mcp_server_started",
            EventMsg::McpServerFailed(_) => "mcp_server_failed",
            EventMsg::McpToolInvoked(_) => "mcp_tool_invoked",
            EventMsg::LspServerStarted(_) => "lsp_server_started",
            EventMsg::LspServerFailed(_) => "lsp_server_failed",
            EventMsg::PlanRequest(_) => "plan_request",
            EventMsg::PlanReady(_) => "plan_ready",
            EventMsg::PlanApproved(_) => "plan_approved",
            EventMsg::PlanRejected(_) => "plan_rejected",
            EventMsg::PermissionModeChanged(_) => "permission_mode_changed",
            EventMsg::Routing(_) => "routing",
        }
    }
}

/// `SystemTime` 的 serde 适配 —— 序列化为 UNIX 秒数,跨进程持久化稳定。
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
        dur.as_secs().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

impl SessionConfiguredEvent {
    pub fn new(model: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            session_id: ThreadId::new(),
            model: model.into(),
            provider: provider.into(),
            approval_policy: ApprovalPolicy::Auto,
            sandbox_policy: SandboxPolicy::WorkspaceOnly,
            context_window_size: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ContentBlock;

    #[test]
    fn all_variants_serialize_with_type_tag() {
        // Spot-check that the `type` tag uses snake_case as the wire format.
        let e = EventMsg::AgentMessageDelta(AgentMessageDelta { delta: "x".into() });
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains(r#""type":"agent_message_delta""#), "got: {j}");

        let e = EventMsg::TurnAborted(TurnAbortedEvent {
            turn_id: TurnId::new(),
            reason: AbortReason::UserInterrupt,
        });
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains(r#""type":"turn_aborted""#), "got: {j}");
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        let msg = EventMsg::ToolCallEnd(ToolCallEndEvent {
            call_id: "c1".into(),
            output: ToolOutput {
                content: vec![ContentBlock::text("ok")],
                is_error: false,
                metadata: serde_json::json!({}),
                elapsed_ms: 42,
            },
            is_error: false,
            elapsed_ms: 42,
        });
        let j = serde_json::to_string(&msg).unwrap();
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::ToolCallEnd(ToolCallEndEvent {
                call_id,
                elapsed_ms,
                ..
            }) => {
                assert_eq!(call_id, "c1");
                assert_eq!(elapsed_ms, 42);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn discriminant_covers_all_variants() {
        let _ = ApprovalPolicy::Auto; // suppress unused warning for this symbol
        let all = vec![
            EventMsg::ShutdownComplete,
            EventMsg::AgentMessage(AgentMessage {
                text: String::new(),
            }),
            EventMsg::AgentMessageDelta(AgentMessageDelta {
                delta: String::new(),
            }),
            EventMsg::ThinkingDelta(ThinkingDelta {
                delta: String::new(),
            }),
            EventMsg::TokenCount(TokenCountEvent {
                input_tokens: 0,
                output_tokens: 0,
                cached_tokens: 0,
                cache_write_tokens: 0,
                total_tokens: 0,
                cost_usd: None,
                ..Default::default()
            }),
            EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: TurnId::new(),
                user_message_id: None,
            }),
            EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id: TurnId::new(),
                usage: TokenUsage::default(),
                status: TurnStatus::Success,
            }),
            EventMsg::TurnAborted(TurnAbortedEvent {
                turn_id: TurnId::new(),
                reason: AbortReason::UserInterrupt,
            }),
            EventMsg::ContextCompacted(ContextCompactedEvent {
                strategy: ContextCompactedStrategy::Noop,
                removed_messages: 0,
                before_tokens: 0,
                after_tokens: 0,
            }),
            EventMsg::Error(ErrorEvent {
                code: "X".into(),
                message: "y".into(),
                details: None,
            }),
            EventMsg::StreamError(StreamErrorEvent {
                code: "X".into(),
                message: "y".into(),
                retry_in_ms: 0,
                ..Default::default()
            }),
            EventMsg::ToolCallBegin(ToolCallBeginEvent {
                call_id: "c".into(),
                tool_name: "t".into(),
                args: serde_json::Value::Null,
            }),
            EventMsg::SessionConfigured(SessionConfiguredEvent::new("m", "p")),
            EventMsg::ApprovalRequest(ApprovalRequestEvent {
                request_id: "r1".into(),
                kind: ApprovalKind::Tool {
                    tool_name: "bash".into(),
                    args: serde_json::json!({"cmd": "ls"}),
                },
                risk: RiskLevel::Medium,
            }),
            EventMsg::AskUserQuestion(AskUserQuestionEvent {
                request_id: "q1".into(),
                questions: vec![crate::question::Question {
                    header: "Lang".into(),
                    question: "Pick a language".into(),
                    options: vec![
                        crate::question::QuestionOption {
                            label: "Rust".into(),
                            description: "safe + fast".into(),
                            preview: None,
                        },
                        crate::question::QuestionOption {
                            label: "Go".into(),
                            description: "simple".into(),
                            preview: None,
                        },
                    ],
                    multi_select: false,
                }],
            }),
            EventMsg::ConfigReloaded(ConfigReloadedEvent {
                path: PathBuf::from("/home/u/.reflect/config.toml"),
                sections_changed: vec!["anthropic".into()],
                at: SystemTime::UNIX_EPOCH,
            }),
            EventMsg::CollabStarted(CollabStartedEvent {
                id: "00000000-0000-0000-0000-000000000000".into(),
                participants: vec!["a".into(), "b".into()],
                mode: "sequential".into(),
            }),
            EventMsg::CollabMessage(CollabMessageEvent {
                id: "00000000-0000-0000-0000-000000000000".into(),
                from: "a".into(),
                kind: "utterance".into(),
                content: "hi".into(),
                round: 0,
                token_usage: None,
            }),
            EventMsg::CollabFinished(CollabFinishedEvent {
                id: "00000000-0000-0000-0000-000000000000".into(),
                outcome: "consensus".into(),
                rounds: 2,
            }),
            EventMsg::McpServerStarted(McpServerStartedEvent {
                server: "fs".into(),
                tool_count: 3,
                tool_names: vec![],
                transport: McpTransportMirror::Stdio,
            }),
            EventMsg::McpServerFailed(McpServerFailedEvent {
                server: "fs".into(),
                error: "spawn: not found".into(),
                will_retry: false,
            }),
            EventMsg::McpToolInvoked(McpToolInvokedEvent {
                server: "fs".into(),
                tool: "read_file".into(),
                call_id: "call_abc".into(),
            }),
            EventMsg::LspServerStarted(LspServerStartedEvent {
                server: "rust".into(),
                methods: vec![
                    "textDocument/definition".to_string(),
                    "textDocument/references".to_string(),
                    "textDocument/hover".to_string(),
                ],
                language_ids: vec!["rust".to_string()],
            }),
            EventMsg::LspServerFailed(LspServerFailedEvent {
                server: "rust".into(),
                error: "spawn: executable not found".into(),
                will_retry: false,
            }),
            EventMsg::PlanRequest(PlanRequestEvent {
                task: "refactor auth".into(),
            }),
            EventMsg::PlanReady(PlanReadyEvent {
                plan_id: PlanId::new(),
                markdown: "## Plan\n1. read auth.rs\n2. edit token validation".into(),
            }),
            EventMsg::PlanApproved(PlanApprovedEvent {
                plan_id: PlanId::new(),
            }),
            EventMsg::PlanRejected(PlanRejectedEvent {
                plan_id: PlanId::new(),
                reason: None,
            }),
            EventMsg::PermissionModeChanged(PermissionModeChangedEvent {
                from: PermissionMode::Auto,
                to: PermissionMode::Plan,
            }),
        ];
        for v in &all {
            assert!(!v.discriminant().is_empty());
        }
        // Verify no two variants collide.
        let mut d: Vec<_> = all.iter().map(|v| v.discriminant()).collect();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), all.len());
    }

    #[test]
    fn approval_request_tool_kind_roundtrip() {
        let ev = EventMsg::ApprovalRequest(ApprovalRequestEvent {
            request_id: "req-7".into(),
            kind: ApprovalKind::Tool {
                tool_name: "bash".into(),
                args: serde_json::json!({"cmd": "rm -rf /tmp/x"}),
            },
            risk: RiskLevel::High,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"approval_request""#), "got: {j}");
        assert!(j.contains(r#""risk":"high""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::ApprovalRequest(e) => {
                assert_eq!(e.request_id, "req-7");
                match e.kind {
                    ApprovalKind::Tool { tool_name, args } => {
                        assert_eq!(tool_name, "bash");
                        assert_eq!(args["cmd"], "rm -rf /tmp/x");
                    }
                    ApprovalKind::Hook { .. } => panic!("wrong kind"),
                    ApprovalKind::Plan { .. } => panic!("wrong kind"),
                }
                assert_eq!(e.risk, RiskLevel::High);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn approval_request_hook_kind_roundtrip() {
        let ev = EventMsg::ApprovalRequest(ApprovalRequestEvent {
            request_id: "req-9".into(),
            kind: ApprovalKind::Hook {
                hook_name: "dangerous_command_blocker".into(),
                decision_preview: "deny: matched 'rm -rf /'".into(),
            },
            risk: RiskLevel::default(),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""risk":"low""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        if let EventMsg::ApprovalRequest(e) = back {
            assert!(matches!(e.kind, ApprovalKind::Hook { .. }));
            assert_eq!(e.risk, RiskLevel::Low);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn risk_level_default_is_low() {
        assert_eq!(RiskLevel::default(), RiskLevel::Low);
    }

    #[test]
    fn approval_request_omitted_risk_deserializes_as_low() {
        // Wire compatibility: a producer that omits the field should land Low.
        let j = r#"{"type":"approval_request","request_id":"r","kind":{"type":"tool","tool_name":"t","args":{}}}"#;
        let back: EventMsg = serde_json::from_str(j).unwrap();
        if let EventMsg::ApprovalRequest(e) = back {
            assert_eq!(e.risk, RiskLevel::Low);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn token_count_with_cost_usd_roundtrip() {
        // M8 P1a: `cost_usd` is `Option<f64>`; both the Some and None
        // branches must roundtrip through serde.
        let ev = EventMsg::TokenCount(TokenCountEvent {
            input_tokens: 1000,
            output_tokens: 200,
            cached_tokens: 50,
            cache_write_tokens: 30,
            total_tokens: 1200,
            cost_usd: Some(0.0009),
            ..Default::default()
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""cost_usd":0.0009"#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::TokenCount(e) => {
                assert_eq!(e.cost_usd, Some(0.0009));
                assert_eq!(e.cache_write_tokens, 30);
                assert_eq!(e.cached_tokens, 50);
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // None branch: must serialize as absent (`skip_serializing_if`).
        let ev_none = EventMsg::TokenCount(TokenCountEvent {
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 0,
            cost_usd: None,
            ..Default::default()
        });
        let j = serde_json::to_string(&ev_none).unwrap();
        assert!(!j.contains("cost_usd"), "None must be skipped, got: {j}");
    }

    #[test]
    fn token_count_backward_compat_with_m7_wire() {
        // M8: M7 producers didn't have `cache_write_tokens` or `cost_usd`.
        // M8 consumers must still accept M7 wire format.
        let j = r#"{"type":"token_count","input_tokens":1,"output_tokens":2,"cached_tokens":3,"total_tokens":3}"#;
        let back: EventMsg = serde_json::from_str(j).unwrap();
        match back {
            EventMsg::TokenCount(e) => {
                assert_eq!(e.input_tokens, 1);
                assert_eq!(e.cached_tokens, 3);
                assert_eq!(e.cache_write_tokens, 0, "missing field defaults to 0");
                assert_eq!(e.cost_usd, None, "missing field defaults to None");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn config_reloaded_roundtrip() {
        use std::time::Duration;
        let ev = EventMsg::ConfigReloaded(ConfigReloadedEvent {
            path: PathBuf::from("/home/u/.reflect/config.toml"),
            sections_changed: vec!["anthropic".into(), "compact".into()],
            at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"config_reloaded""#), "got: {j}");
        assert!(
            j.contains(r#""sections_changed":["anthropic","compact"]"#),
            "got: {j}"
        );
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::ConfigReloaded(e) => {
                assert_eq!(e.path, PathBuf::from("/home/u/.reflect/config.toml"));
                assert_eq!(
                    e.sections_changed,
                    vec!["anthropic".to_string(), "compact".to_string()]
                );
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── M10/v0.2.4: Collab* events ────────────────────────────────────────

    #[test]
    fn collab_started_roundtrip() {
        let ev = EventMsg::CollabStarted(CollabStartedEvent {
            id: "11111111-2222-3333-4444-555555555555".into(),
            participants: vec!["advocate".into(), "skeptic".into(), "moderator".into()],
            mode: "sequential".into(),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"collab_started""#), "got: {j}");
        assert!(
            j.contains(r#""id":"11111111-2222-3333-4444-555555555555""#),
            "got: {j}"
        );
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::CollabStarted(e) => {
                assert_eq!(e.id, "11111111-2222-3333-4444-555555555555");
                assert_eq!(e.participants.len(), 3);
                assert_eq!(e.mode, "sequential");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn collab_message_with_token_usage_roundtrip() {
        let ev = EventMsg::CollabMessage(CollabMessageEvent {
            id: "abc".into(),
            from: "advocate".into(),
            kind: "utterance".into(),
            content: "I disagree".into(),
            round: 2,
            token_usage: Some(TokenUsage::new(123, 45, 10)),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"collab_message""#), "got: {j}");
        assert!(j.contains(r#""round":2"#), "got: {j}");
        assert!(j.contains(r#""token_usage""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::CollabMessage(e) => {
                assert_eq!(e.id, "abc");
                assert_eq!(e.from, "advocate");
                assert_eq!(e.kind, "utterance");
                assert_eq!(e.round, 2);
                let u = e.token_usage.expect("token_usage roundtrips Some");
                assert_eq!(u.input_tokens, 123);
                assert_eq!(u.output_tokens, 45);
                assert_eq!(u.cached_tokens, 10);
                assert_eq!(u.total_tokens, 168);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn collab_message_without_token_usage_omits_field() {
        let ev = EventMsg::CollabMessage(CollabMessageEvent {
            id: "abc".into(),
            from: "advocate".into(),
            kind: "utterance".into(),
            content: "hi".into(),
            round: 0,
            token_usage: None,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(
            !j.contains("token_usage"),
            "None must be skipped via skip_serializing_if, got: {j}"
        );
        // And round-trip still decodes as None.
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        if let EventMsg::CollabMessage(e) = back {
            assert!(e.token_usage.is_none());
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn collab_finished_roundtrip() {
        let ev = EventMsg::CollabFinished(CollabFinishedEvent {
            id: "11111111-2222-3333-4444-555555555555".into(),
            outcome: "consensus".into(),
            rounds: 4,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"collab_finished""#), "got: {j}");
        assert!(j.contains(r#""outcome":"consensus""#), "got: {j}");
        assert!(j.contains(r#""rounds":4"#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::CollabFinished(e) => {
                assert_eq!(e.id, "11111111-2222-3333-4444-555555555555");
                assert_eq!(e.outcome, "consensus");
                assert_eq!(e.rounds, 4);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── MCP (v0.3) ───────────────────────────────────────────────────────

    #[test]
    fn mcp_server_started_roundtrip() {
        // stdio transport。
        let ev = EventMsg::McpServerStarted(McpServerStartedEvent {
            server: "filesystem".into(),
            tool_count: 7,
            tool_names: vec![],
            transport: McpTransportMirror::Stdio,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"mcp_server_started""#), "got: {j}");
        assert!(j.contains(r#""server":"filesystem""#), "got: {j}");
        assert!(j.contains(r#""tool_count":7"#), "got: {j}");
        assert!(j.contains(r#""transport":"stdio""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::McpServerStarted(e) => {
                assert_eq!(e.server, "filesystem");
                assert_eq!(e.tool_count, 7);
                assert_eq!(e.transport, McpTransportMirror::Stdio);
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // http transport 也覆盖到。
        let ev_http = EventMsg::McpServerStarted(McpServerStartedEvent {
            server: "notion".into(),
            tool_count: 12,
            tool_names: vec![],
            transport: McpTransportMirror::Http,
        });
        let j_http = serde_json::to_string(&ev_http).unwrap();
        assert!(j_http.contains(r#""transport":"http""#), "got: {j_http}");
    }

    #[test]
    fn mcp_server_failed_roundtrip() {
        let ev = EventMsg::McpServerFailed(McpServerFailedEvent {
            server: "broken".into(),
            error: "spawn: executable not found".into(),
            will_retry: false,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"mcp_server_failed""#), "got: {j}");
        assert!(j.contains(r#""server":"broken""#), "got: {j}");
        assert!(j.contains(r#""will_retry":false"#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::McpServerFailed(e) => {
                assert_eq!(e.server, "broken");
                assert!(e.error.contains("spawn"));
                assert!(!e.will_retry);
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // will_retry=true 路径(http 重连循环)也走通。
        let ev_retry = EventMsg::McpServerFailed(McpServerFailedEvent {
            server: "flaky".into(),
            error: "connection reset".into(),
            will_retry: true,
        });
        let j_retry = serde_json::to_string(&ev_retry).unwrap();
        assert!(j_retry.contains(r#""will_retry":true"#), "got: {j_retry}");
    }

    #[test]
    fn mcp_tool_invoked_roundtrip() {
        let ev = EventMsg::McpToolInvoked(McpToolInvokedEvent {
            server: "filesystem".into(),
            tool: "read_file".into(),
            call_id: "toolu_01A".into(),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"mcp_tool_invoked""#), "got: {j}");
        assert!(j.contains(r#""server":"filesystem""#), "got: {j}");
        assert!(j.contains(r#""tool":"read_file""#), "got: {j}");
        assert!(j.contains(r#""call_id":"toolu_01A""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::McpToolInvoked(e) => {
                assert_eq!(e.server, "filesystem");
                assert_eq!(e.tool, "read_file");
                assert_eq!(e.call_id, "toolu_01A");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── Plan mode events (v1.x) ──────────────────────────────────────────

    #[test]
    fn plan_request_event_roundtrip() {
        let ev = EventMsg::PlanRequest(PlanRequestEvent {
            task: "refactor auth module".into(),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"plan_request""#), "got: {j}");
        assert!(j.contains(r#""task":"refactor auth module""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::PlanRequest(e) => assert_eq!(e.task, "refactor auth module"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn plan_ready_event_roundtrip() {
        let pid = PlanId::new();
        let ev = EventMsg::PlanReady(PlanReadyEvent {
            plan_id: pid,
            markdown: "## Plan\n- step 1\n- step 2".into(),
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"plan_ready""#), "got: {j}");
        assert!(j.contains(r#""plan_id""#), "got: {j}");
        assert!(j.contains(r#""markdown""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::PlanReady(e) => {
                assert_eq!(e.plan_id, pid);
                assert!(e.markdown.contains("step 1"));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn plan_approved_event_roundtrip() {
        let pid = PlanId::new();
        let ev = EventMsg::PlanApproved(PlanApprovedEvent { plan_id: pid });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"plan_approved""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::PlanApproved(e) => assert_eq!(e.plan_id, pid),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn plan_rejected_event_omits_none_reason() {
        // wire 兼容性:reason: None 必须 skip_serializing_if,不能占位
        let pid = PlanId::new();
        let ev_none = EventMsg::PlanRejected(PlanRejectedEvent {
            plan_id: pid,
            reason: None,
        });
        let j_none = serde_json::to_string(&ev_none).unwrap();
        assert!(!j_none.contains("reason"), "None 应跳过,got: {j_none}");

        // Some 路径也走通。
        let ev_some = EventMsg::PlanRejected(PlanRejectedEvent {
            plan_id: pid,
            reason: Some("plan 风险太大".into()),
        });
        let j_some = serde_json::to_string(&ev_some).unwrap();
        assert!(
            j_some.contains(r#""reason":"plan 风险太大""#),
            "got: {j_some}"
        );

        // roundtrip: Some / None 都正确还原。
        let back_some: EventMsg = serde_json::from_str(&j_some).unwrap();
        if let EventMsg::PlanRejected(e) = back_some {
            assert_eq!(e.reason.as_deref(), Some("plan 风险太大"));
        } else {
            panic!("wrong variant");
        }
        let back_none: EventMsg = serde_json::from_str(&j_none).unwrap();
        if let EventMsg::PlanRejected(e) = back_none {
            assert!(e.reason.is_none());
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn permission_mode_changed_event_roundtrip() {
        let ev = EventMsg::PermissionModeChanged(PermissionModeChangedEvent {
            from: PermissionMode::Auto,
            to: PermissionMode::Plan,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(
            j.contains(r#""type":"permission_mode_changed""#),
            "got: {j}"
        );
        // PermissionMode::Plan 用 snake_case 序列化为 "plan"。
        assert!(j.contains(r#""to":"plan""#), "got: {j}");
        assert!(j.contains(r#""from":"auto""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::PermissionModeChanged(e) => {
                assert_eq!(e.from, PermissionMode::Auto);
                assert_eq!(e.to, PermissionMode::Plan);
            }
            other => panic!("wrong variant: {other:?}"),
        }

        // 退出路径(Plan → Prompt)。
        let back_to_prompt = EventMsg::PermissionModeChanged(PermissionModeChangedEvent {
            from: PermissionMode::Plan,
            to: PermissionMode::Prompt,
        });
        let j_back = serde_json::to_string(&back_to_prompt).unwrap();
        assert!(j_back.contains(r#""from":"plan""#), "got: {j_back}");
        assert!(j_back.contains(r#""to":"prompt""#), "got: {j_back}");
    }

    #[test]
    fn approval_kind_plan_roundtrip() {
        let pid = PlanId::new();
        let ev = EventMsg::ApprovalRequest(ApprovalRequestEvent {
            request_id: "req-plan-1".into(),
            kind: ApprovalKind::Plan {
                plan_id: pid,
                summary: "Refactor auth module: split into 3 files".into(),
            },
            risk: RiskLevel::Medium,
        });
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"plan""#), "Plan kind tag: got: {j}");
        assert!(j.contains(r#""plan_id""#), "got: {j}");
        assert!(j.contains(r#""summary""#), "got: {j}");
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::ApprovalRequest(e) => match e.kind {
                ApprovalKind::Plan { plan_id, summary } => {
                    assert_eq!(plan_id, pid);
                    assert!(summary.contains("Refactor"));
                }
                _ => panic!("wrong kind"),
            },
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── v1.1.0: AskUserQuestion ──────────────────────────────────────

    #[test]
    fn ask_user_question_event_roundtrip() {
        use crate::question::{Question, QuestionOption};
        let q = Question {
            header: "Lang".into(),
            question: "Pick a language".into(),
            options: vec![
                QuestionOption {
                    label: "Rust".into(),
                    description: "safe + fast".into(),
                    preview: None,
                },
                QuestionOption {
                    label: "Go".into(),
                    description: "simple".into(),
                    preview: Some("```\nfn main() {}\n```".into()),
                },
            ],
            multi_select: false,
        };
        let ev = EventMsg::AskUserQuestion(AskUserQuestionEvent::new("q1", vec![q]));
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""type":"ask_user_question""#), "got: {j}");
        assert!(j.contains(r#""request_id":"q1""#), "got: {j}");
        assert!(j.contains(r#""header":"Lang""#), "got: {j}");
        assert!(j.contains(r#""multi_select":false"#), "got: {j}");

        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::AskUserQuestion(e) => {
                assert_eq!(e.request_id, "q1");
                assert_eq!(e.questions.len(), 1);
                assert_eq!(e.questions[0].header, "Lang");
                assert_eq!(e.questions[0].options.len(), 2);
                assert!(e.questions[0].options[1].preview.is_some());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn ask_user_question_multi_question_roundtrip() {
        use crate::question::{Question, QuestionOption};
        let qs = vec![
            Question {
                header: "Lang".into(),
                question: "Pick a language".into(),
                options: vec![
                    QuestionOption {
                        label: "Rust".into(),
                        description: "safe + fast".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "Go".into(),
                        description: "simple".into(),
                        preview: None,
                    },
                ],
                multi_select: false,
            },
            Question {
                header: "Deploy".into(),
                question: "Where to deploy?".into(),
                options: vec![
                    QuestionOption {
                        label: "AWS".into(),
                        description: "managed".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "GCP".into(),
                        description: "managed".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "Self".into(),
                        description: "BYO infra".into(),
                        preview: None,
                    },
                ],
                multi_select: true,
            },
        ];
        let ev = EventMsg::AskUserQuestion(AskUserQuestionEvent::new("multi", qs));
        let j = serde_json::to_string(&ev).unwrap();
        let back: EventMsg = serde_json::from_str(&j).unwrap();
        match back {
            EventMsg::AskUserQuestion(e) => {
                assert_eq!(e.questions.len(), 2);
                assert!(!e.questions[0].multi_select);
                assert!(e.questions[1].multi_select);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }
}
