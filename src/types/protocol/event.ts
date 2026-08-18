/**
 * Reflect protocol —— Tauri `reflect_event` 信封。
 *
 * 对应 `reflect-agent/crates/protocol/reflect-protocol/src/event.rs` 和 `event_msg.rs`。
 * 外层信封形状为 `{ id: string; msg: ReflectEventMsg }`。`EventMsgByType` 映射
 * 是 Rust `EventMsg` 枚举的类型层镜像,允许 TS 通过 `event.msg.type` 收窄类型。
 *
 * 从 barrel 再导出的辅助/常量:
 *   - `EVENT_ID_NONE` → `''`(空字符串,session 级事件的 id)
 *   - `EventMsgType` → 所有 discriminator 值的字符串字面量联合
 *   - `ReflectEventMsg` → 跨 `EventMsgType` 的判别联合
 *   - `ReflectEvent` → 外层信封
 *
 * 顶层同步警告见 `./index.ts`。
 */

import type { ApprovalPolicy, RiskLevel, SandboxPolicy, PermissionMode } from './enums';
import type { AskUserInputPayload } from './ask_user_input';
import type { Question } from './question';
import type { TokenUsagePayload } from './usage';

/** Submission id `"EVENT_ID_NONE"` = session 级事件,没有对应的 submission。 */
export const EVENT_ID_NONE = '';

/**
 * 外层信封:Tauri `reflect_event` 事件的 payload 形如
 * `{ id: string; msg: EventMsgByType[T] }`。`Event` 接口基于 `msg.type` 使用
 * TypeScript 的判别联合,使消费者可以用 `switch (event.msg.type)` 收窄 payload 形状。
 */
export type EventMsgType =
  // 生命周期 (6)
  | 'session_configured'
  | 'turn_started'
  | 'turn_complete'
  | 'turn_aborted'
  | 'turn_rewound'
  | 'shutdown_complete'
  // LLM 输出 (4)
  | 'agent_message'
  | 'agent_message_delta'
  | 'thinking_delta'
  | 'token_count'
  // 工具 (2)
  | 'tool_call_begin'
  | 'tool_call_end'
  // 审批 (1)
  | 'approval_request'
  // 用户提问 (2)
  | 'ask_user_question'
  | 'ask_user_input'
  // 权限气泡 (1)
  | 'permission_bubble'
  // 上下文压缩 (1)
  | 'context_compacted'
  // 错误 (2)
  | 'error'
  | 'stream_error'
  // 配置 / 路由 (2)
  | 'config_reloaded'
  | 'routing'
  // 协作 (3)
  | 'collab_started'
  | 'collab_message'
  | 'collab_finished'
  // MCP (3)
  | 'mcp_server_started'
  | 'mcp_server_failed'
  | 'mcp_tool_invoked'
  // LSP (2)
  | 'lsp_server_started'
  | 'lsp_server_failed'
  // Plan 模式 (5)
  | 'plan_request'
  | 'plan_ready'
  | 'plan_approved'
  | 'plan_rejected'
  | 'plan_step'
  | 'permission_mode_changed'
  // 插件 / 配额
  | 'plugin_loaded'
  | 'quota_exhausted';

// ============================================================================
// EventMsg payload 结构体
// ============================================================================

export interface SessionConfiguredPayload {
  session_id: string; // ThreadId (uuid)
  model: string;
  provider: string;
  approval_policy: ApprovalPolicy;
  sandbox_policy: SandboxPolicy;
  context_window_size?: number;
}

export interface TurnStartedPayload {
  turn_id: string; // TurnId
  user_message_id?: string;
}

export interface TurnCompletePayload {
  turn_id: string;
  usage: TokenUsagePayload;
  status: 'success' | 'max_iterations' | 'stopped' | 'token_budget_exceeded';
}

export type AbortReasonPayload =
  | { type: 'user_interrupt' }
  | { type: 'shutdown' }
  | { type: 'error'; code: string; message: string };

export interface TurnAbortedPayload {
  turn_id: string;
  reason: AbortReasonPayload;
}

export interface TurnRewoundPayload {
  to_turn_id?: string;
  truncated_after: number;
}

export interface AgentMessagePayload {
  text: string;
}

export interface AgentMessageDeltaPayload {
  delta: string;
}

export interface ThinkingDeltaPayload {
  delta: string;
}

export interface TokenCountPayload extends TokenUsagePayload {
  // 与 TokenUsagePayload 结构一致
}

export interface ToolCallBeginPayload {
  call_id: string;
  tool_name: string;
  args: unknown;
}

export interface ToolCallEndPayload {
  call_id: string;
  output: ToolOutput;
  is_error: boolean;
  elapsed_ms: number;
}

export interface ToolOutput {
  content: ContentBlock[];
  is_error: boolean;
  metadata: unknown;
  elapsed_ms: number;
}

export interface ContentBlock {
  type: 'text' | 'image' | 'diff' | 'tool_use' | 'tool_result';
  // 文本
  text?: string;
  // 图像
  data?: string; // base64
  mime_type?: string;
  // 差异
  unified_diff?: string;
  // 工具调用
  id?: string;
  name?: string;
  args?: unknown;
  // 工具结果
  call_id?: string;
  output?: ToolOutput;
}

export interface ApprovalRequestPayload {
  request_id: string;
  kind: ApprovalKind;
  risk?: RiskLevel;
}

export type ApprovalKind =
  | { type: 'tool'; tool_name: string; args: unknown }
  | { type: 'hook'; hook_name: string; decision_preview: string }
  | { type: 'plan'; plan_id: string; summary: string };

export interface AskUserQuestionPayload {
  request_id: string;
  questions: Question[];
}

export interface PermissionBubblePayload {
  tool_name: string;
  args_preview?: string;
  risk?: RiskLevel;
}

export interface ContextCompactedPayload {
  strategy: 'noop' | 'microcompact' | 'smart_prune' | 'llm_summarize';
  removed_messages: number;
  before_tokens: number;
  after_tokens: number;
}

export interface ErrorPayload {
  code: string;
  message: string;
  details?: unknown;
}

export interface StreamErrorPayload {
  code: string;
  message: string;
  retry_in_ms: number;
  provider?: string;
  credential_label?: string;
  tried?: { label: string; outcome: string }[];
}

export interface ConfigReloadedPayload {
  path: string;
  sections_changed: string[];
  at: number; // Unix 时间戳（秒）
}

export interface RoutingPayload {
  kind: 'switched' | 'failed_over' | 'cooldown_started' | 'cooldown_cleared';
  role: string;
  from_credential?: string;
  to_credential?: string;
  reason: string;
  cooldown_until_ms?: number;
}

export interface CollabStartedPayload {
  id: string;
  participants: string[];
  mode: string;
}

export interface CollabMessagePayload {
  id: string;
  from: string;
  kind: string; // "utterance" | "consensus" | "finish"
  content: string;
  round: number;
  token_usage?: TokenUsagePayload;
}

export interface CollabFinishedPayload {
  id: string;
  outcome: string; // "consensus" | "no_consensus" | "finished"
  rounds: number;
}

export interface McpServerStartedPayload {
  server: string;
  tool_count: number;
  tool_names?: string[];
  transport: 'stdio' | 'http' | 'sse';
}

export interface McpServerFailedPayload {
  server: string;
  error: string;
  will_retry: boolean;
}

export interface McpToolInvokedPayload {
  server: string;
  tool: string;
  call_id: string;
}

export interface LspServerStartedPayload {
  server: string;
  methods: string[];
  language_ids: string[];
}

export interface LspServerFailedPayload {
  server: string;
  error: string;
  will_retry: boolean;
}

export interface PlanRequestPayload {
  task: string;
}

export interface PlanReadyPayload {
  plan_id: string;
  markdown: string;
}

export interface PlanApprovedPayload {
  plan_id: string;
}

export interface PlanRejectedPayload {
  plan_id: string;
  reason?: string;
}

export interface PermissionModeChangedPayload {
  from: PermissionMode;
  to: PermissionMode;
}

/** PlanStepStatus —— snake_case(对齐 Rust `PlanStepStatus`)。 */
export type PlanStepStatus = 'pending' | 'in_progress' | 'done' | 'skipped';

/** `plan_step` —— plan 执行进度(单 step 状态变更)。 */
export interface PlanStepPayload {
  plan_id: string;
  index: number;
  /** 已知 step 总数(0 = 未知 / 动态)。 */
  total: number;
  status: PlanStepStatus;
  title?: string;
}

/** `plugin_loaded` —— 插件运行时挂载完成。 */
export interface PluginLoadedPayload {
  plugin: string;
  scope?: string;
  version?: string;
  skill_count?: number;
}

/** `quota_exhausted` —— credential 配额耗尽,agent 因此停止。 */
export interface QuotaExhaustedPayload {
  provider: string;
  label: string;
  used_tokens?: number;
  max_tokens?: number;
  /** 窗口结束的 unix 秒数(窗口重置时间,后端 `QuotaExhaustedEvent::window_ends_secs`)。 */
  window_ends_secs?: number;
}

// ============================================================================
// EventMsgByType —— 判别联合
// ============================================================================

/**
 * `EventMsgType` → payload 形状的映射。这是 Rust `EventMsg` 枚举的类型层镜像,
 * 让 TS 可以通过 `event.msg.type` 收窄。
 */
export interface EventMsgByType {
  // 生命周期
  session_configured: SessionConfiguredPayload;
  turn_started: TurnStartedPayload;
  turn_complete: TurnCompletePayload;
  turn_aborted: TurnAbortedPayload;
  turn_rewound: TurnRewoundPayload;
  shutdown_complete: Record<string, never>;
  // LLM 输出
  agent_message: AgentMessagePayload;
  agent_message_delta: AgentMessageDeltaPayload;
  thinking_delta: ThinkingDeltaPayload;
  token_count: TokenCountPayload;
  // 工具
  tool_call_begin: ToolCallBeginPayload;
  tool_call_end: ToolCallEndPayload;
  // 审批
  approval_request: ApprovalRequestPayload;
  ask_user_question: AskUserQuestionPayload;
  ask_user_input: AskUserInputPayload;
  permission_bubble: PermissionBubblePayload;
  // 上下文压缩
  context_compacted: ContextCompactedPayload;
  // 错误
  error: ErrorPayload;
  stream_error: StreamErrorPayload;
  // 配置 / 路由
  config_reloaded: ConfigReloadedPayload;
  routing: RoutingPayload;
  // 协作
  collab_started: CollabStartedPayload;
  collab_message: CollabMessagePayload;
  collab_finished: CollabFinishedPayload;
  // MCP
  mcp_server_started: McpServerStartedPayload;
  mcp_server_failed: McpServerFailedPayload;
  mcp_tool_invoked: McpToolInvokedPayload;
  // LSP
  lsp_server_started: LspServerStartedPayload;
  lsp_server_failed: LspServerFailedPayload;
  // Plan
  plan_request: PlanRequestPayload;
  plan_ready: PlanReadyPayload;
  plan_approved: PlanApprovedPayload;
  plan_rejected: PlanRejectedPayload;
  plan_step: PlanStepPayload;
  permission_mode_changed: PermissionModeChangedPayload;
  // 插件 / 配额
  plugin_loaded: PluginLoadedPayload;
  quota_exhausted: QuotaExhaustedPayload;
}

/** ReflectEvent 的单一联合变体 —— 通过 `msg.type` 收窄。 */
export type ReflectEventMsg = {
  [T in EventMsgType]: { type: T } & EventMsgByType[T];
}[EventMsgType];

/** 外层 Tauri `reflect_event` payload。 */
export interface ReflectEvent {
  id: string;
  msg: ReflectEventMsg;
}
