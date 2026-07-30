/**
 * Reflect protocol — Tauri `reflect_event` envelope.
 *
 * Mirrors `vendor/reflect-protocol/src/event.rs` and `event_msg.rs`.
 * Outer envelope shape is `{ id: string; msg: ReflectEventMsg }`. The
 * `EventMsgByType` map is the type-level analog of the Rust `EventMsg`
 * enum, letting TS narrow via `event.msg.type`.
 *
 * Helpers / constants re-exported from the barrel:
 *   - `EVENT_ID_NONE` → `''` (empty string — id of session-level events)
 *   - `EventMsgType` → string literal union of all discriminator values
 *   - `ReflectEventMsg` → discriminated union over `EventMsgType`
 *   - `ReflectEvent` → outer envelope
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

import type { ApprovalPolicy, RiskLevel, SandboxPolicy, PermissionMode } from './enums';
import type { AskUserInputPayload } from './ask_user_input';
import type { Question } from './question';
import type { TokenUsagePayload } from './usage';

/** Submission id `"EVENT_ID_NONE"` = session-level event, no matching submission. */
export const EVENT_ID_NONE = '';

/**
 * Outer envelope: Tauri event `reflect_event` payload is exactly
 * `{ id: string; msg: EventMsgByType[T] }`.  The `Event` interface uses
 * TypeScript's discriminated union over `msg.type` so consumers can
 * narrow payload shape with `switch (event.msg.type)`.
 */
export type EventMsgType =
  // Lifecycle (6)
  | 'session_configured'
  | 'turn_started'
  | 'turn_complete'
  | 'turn_aborted'
  | 'turn_rewound'
  | 'shutdown_complete'
  // LLM output (4)
  | 'agent_message'
  | 'agent_message_delta'
  | 'thinking_delta'
  | 'token_count'
  // Tool (2)
  | 'tool_call_begin'
  | 'tool_call_end'
  // Approval (1)
  | 'approval_request'
  // AskUser (2)
  | 'ask_user_question'
  | 'ask_user_input'
  // Permission bubble (1)
  | 'permission_bubble'
  // Compaction (1)
  | 'context_compacted'
  // Error (2)
  | 'error'
  | 'stream_error'
  // Config / routing (2)
  | 'config_reloaded'
  | 'routing'
  // Collab (3)
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
  // Plan mode (5)
  | 'plan_request'
  | 'plan_ready'
  | 'plan_approved'
  | 'plan_rejected'
  | 'plan_step'
  | 'permission_mode_changed'
  // Plugin / quota
  | 'plugin_loaded'
  | 'quota_exhausted';

// ============================================================================
// EventMsg payload structs
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
  // identical to TokenUsagePayload
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
  // text
  text?: string;
  // image
  data?: string; // base64
  mime_type?: string;
  // diff
  unified_diff?: string;
  // tool_use
  id?: string;
  name?: string;
  args?: unknown;
  // tool_result
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
  at: number; // unix seconds
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

/** PlanStepStatus — snake_case(对齐 Rust `PlanStepStatus`)。 */
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
// EventMsgByType — discriminated union
// ============================================================================

/**
 * Map of `EventMsgType` → payload shape. This is the type-level analog of
 * the Rust `EventMsg` enum, allowing TS to narrow via `event.msg.type`.
 */
export interface EventMsgByType {
  // Lifecycle
  session_configured: SessionConfiguredPayload;
  turn_started: TurnStartedPayload;
  turn_complete: TurnCompletePayload;
  turn_aborted: TurnAbortedPayload;
  turn_rewound: TurnRewoundPayload;
  shutdown_complete: Record<string, never>;
  // LLM output
  agent_message: AgentMessagePayload;
  agent_message_delta: AgentMessageDeltaPayload;
  thinking_delta: ThinkingDeltaPayload;
  token_count: TokenCountPayload;
  // Tool
  tool_call_begin: ToolCallBeginPayload;
  tool_call_end: ToolCallEndPayload;
  // Approval
  approval_request: ApprovalRequestPayload;
  ask_user_question: AskUserQuestionPayload;
  ask_user_input: AskUserInputPayload;
  permission_bubble: PermissionBubblePayload;
  // Compaction
  context_compacted: ContextCompactedPayload;
  // Error
  error: ErrorPayload;
  stream_error: StreamErrorPayload;
  // Config / routing
  config_reloaded: ConfigReloadedPayload;
  routing: RoutingPayload;
  // Collab
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
  // Plugin / quota
  plugin_loaded: PluginLoadedPayload;
  quota_exhausted: QuotaExhaustedPayload;
}

/** Single union variant for ReflectEvent — narrowed by `msg.type`. */
export type ReflectEventMsg = {
  [T in EventMsgType]: { type: T } & EventMsgByType[T];
}[EventMsgType];

/** Outer Tauri `reflect_event` payload. */
export interface ReflectEvent {
  id: string;
  msg: ReflectEventMsg;
}
