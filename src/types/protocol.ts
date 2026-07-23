/**
 * Reflect protocol TypeScript types — comprehensive mirror of
 * `vendor/reflect-protocol` Rust types.
 *
 * This file is **manually synchronized** with the Rust schema for the
 * Tauri IPC envelope (`Submission`, `Op`, `Event`, `EventMsg`, `UserInputItem`,
 * `ContentBlock`, plus every payload struct). The Rust side now derives
 * `JsonSchema` and the `dump_schema` example + `scripts/dump-ts-types.sh`
 * can regenerate it; for offline development this file is the source of
 * truth and must be updated when adding a new `EventMsg` variant or `Op`
 * variant upstream.
 *
 * ## Invariants
 * 1. `id: string` is the per-submission correlation id (`""` for
 *    `EVENT_ID_NONE` lifecycle events).
 * 2. `msg.type` is the snake_case discriminator; TS narrowing is supported
 *    via the `EventMsgByType` map + `ReflectEvent` discriminated union below.
 * 3. All `UserInputItem` variants match the Rust enum.
 * 4. `ContentBlock` matches the Rust tool-output enum.
 * 5. `ReviewDecision` is a snake_case-tagged enum; `Deny` carries a
 *    `{ reason: string }` payload (NOT PascalCase).
 */

// ============================================================================
// Wire envelope
// ============================================================================

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
  | 'permission_mode_changed';

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

export interface TokenUsagePayload {
  input_tokens: number;
  output_tokens: number;
  cached_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd?: number;
  provider?: string;
  credential_label?: string;
}

export interface TurnCompletePayload {
  turn_id: string;
  usage: TokenUsagePayload;
  status: 'success' | 'max_iterations' | 'stopped' | 'token_budget_exceeded';
}

export interface AbortReasonPayload {
  type: 'user_interrupt' | 'shutdown' | { error: { code: string; message: string } };
}

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

export interface AskUserInputPayload {
  request_id: string;
  prompt: string;
}

export interface Question {
  header: string;
  question: string;
  options: QuestionOption[];
  multi_select: boolean;
}

export interface QuestionOption {
  label: string;
  description: string;
  preview?: string;
}

export interface Answer {
  selected?: number[];
  custom?: string;
}

export interface AskUserAnswer {
  answers: Answer[];
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

// ============================================================================
// Enums (string unions)
// ============================================================================

export type PermissionMode =
  | 'auto'
  | 'prompt'
  | 'deny'
  | 'plan'
  | 'accept_edits'
  | 'bubble'
  | 'bypass';

export type ApprovalPolicy = 'auto' | 'prompt' | 'deny';
export type SandboxPolicy = 'workspace_only' | 'os_sandbox' | 'full_access';
export type RiskLevel = 'low' | 'medium' | 'high';
export type ReasoningEffort = 'low' | 'medium' | 'high';

/** ReviewDecision is snake_case-tagged. Deny carries `{ reason: string }`. */
export type ReviewDecision = 'approve' | { deny: { reason: string } } | 'approve_for_session';

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
  permission_mode_changed: PermissionModeChangedPayload;
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

// ============================================================================
// Submission / Op / UserInputItem
// ============================================================================

export type OpType =
  | 'user_input'
  | 'compact'
  | 'interrupt'
  | 'rewind'
  | 'shutdown'
  | 'tool_approval'
  | 'hook_approval'
  | 'enter_plan_mode'
  | 'exit_plan_mode'
  | 'plan_approval'
  | 'set_effort'
  | 'ask_user_question_response'
  | 'ask_user_input_response'
  | 'set_permission_mode'
  | 'cycle_permission_mode';

export type UserInputItem =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mime_type: string }
  | { type: 'local_image'; path: string }
  | { type: 'skill'; name: string; args?: unknown }
  | { type: 'question_answer'; request_id: string; answers: AskUserAnswer };

export interface ThreadSettingsOverrides {
  model?: string;
  approval_policy?: ApprovalPolicy;
  sandbox_policy?: SandboxPolicy;
  max_tool_concurrency?: number;
}

export interface W3cTraceContext {
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
  trace_flags?: string;
}

export type ReflectSubmissionOp =
  | { type: 'user_input'; items: UserInputItem[]; thread_settings?: ThreadSettingsOverrides }
  | { type: 'interrupt' }
  | { type: 'shutdown' }
  | { type: 'compact' }
  | { type: 'rewind'; to_turn_id?: string | null }
  | { type: 'tool_approval'; id: string; decision: ReviewDecision }
  | { type: 'hook_approval'; id: string; decision: ReviewDecision }
  | { type: 'enter_plan_mode'; task: string }
  | { type: 'exit_plan_mode' }
  | { type: 'plan_approval'; id: string; decision: ReviewDecision }
  | { type: 'set_effort'; effort: ReasoningEffort }
  | { type: 'ask_user_question_response'; id: string; answers: AskUserAnswer }
  | { type: 'ask_user_input_response'; id: string; text: string }
  | { type: 'set_permission_mode'; mode: PermissionMode }
  | { type: 'cycle_permission_mode' };

export interface ReflectSubmission {
  id: string;
  op: ReflectSubmissionOp;
  client_user_message_id?: string;
  trace?: W3cTraceContext;
}

// ============================================================================
// Backward-compat aliases (used by older code that imports these names)
// ============================================================================

export type ReflectEventType = EventMsgType;
export type ReflectSubmissionOpType = OpType;
export type ReflectUserInputItem = UserInputItem;

// ============================================================================
// ApprovalRequest and SessionInfo (rollout)
// ============================================================================

export interface ReflectSessionInfo {
  thread_id: string;
  name?: string;
  created_at: number;
  updated_at: number;
  message_count: number;
  token_total: number;
}

export interface ReflectRolloutRecord {
  seq: number;
  kind: 'submission' | 'event';
  timestamp: number;
  payload: ReflectSubmission | ReflectEvent;
}
