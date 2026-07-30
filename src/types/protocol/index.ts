/**
 * Reflect protocol — barrel re-export.
 *
 * The actual type definitions live in the per-domain modules under
 * `./{event,submission,op,item,question,ask_user_input,rollout,enums,usage}.ts`.
 * This file is the single import target for the rest of the codebase
 * (typically via `@/types/protocol`); the parent `src/types/protocol.ts`
 * file is a thin shim that re-exports everything from here.
 *
 * ## Vendor sync warning
 *
 * This directory is **manually synchronized** with `vendor/reflect-protocol`
 * (the Rust schema for the Tauri IPC envelope). Any drift between the
 * per-domain modules and the Rust types produces a wire-format bug:
 *   - `ReflectEvent` / `ReflectEventMsg` ↔ `serde(rename_all = "snake_case")`
 *   - `ReflectSubmission` / `ReflectSubmissionOp` ↔ `Submission` / `Op`
 *   - `UserInputItem` ↔ `UserInputItem`
 *   - `Question` / `QuestionOption` / `Answer` / `AskUserAnswer` ↔
 *     `Question` / `QuestionOption` / `Answer` / `AskUserAnswer`
 *   - `AskUserInputPayload` ↔ `AskUserInputMsg`
 *   - `ReflectSessionInfo` / `ReflectRolloutRecord` ↔ `SessionInfo` / `RolloutRecord`
 *
 * ## Update workflow
 * 1. Edit/add the Rust enum/variant in `vendor/reflect-protocol/src/...`.
 * 2. Mirror it here in the relevant per-domain module (preserving
 *    snake_case field names, discriminators, and the `{ deny: { reason } }`
 *    shape of `ReviewDecision::Deny`).
 * 3. If the change touches a new event type, add it to `EventMsgType`,
 *    `EventMsgByType`, and the `ReflectEventMsg` mapped type in `event.ts`.
 * 4. Run `pnpm typecheck` and `pnpm test` — must stay green.
 * 5. The `scripts/dump-ts-types.sh` (Rust `dump_schema` example) can
 *    regenerate these from the JSON Schemas, but for offline dev this
 *    directory is the source of truth.
 *
 * ## Invariants
 * 1. `id: string` is the per-submission correlation id (`""` for
 *    `EVENT_ID_NONE` lifecycle events).
 * 2. `msg.type` is the snake_case discriminator; TS narrowing is supported
 *    via the `EventMsgByType` map + `ReflectEvent` discriminated union
 *    in `event.ts`.
 * 3. All `UserInputItem` variants match the Rust enum.
 * 4. `ContentBlock` matches the Rust tool-output enum.
 * 5. `ReviewDecision` is a snake_case-tagged enum; `Deny` carries a
 *    `{ reason: string }` payload (NOT PascalCase).
 */

// ----- Wire envelope / EventMsg -----
export {
  EVENT_ID_NONE,
  type EventMsgType,
  type EventMsgByType,
  type ReflectEvent,
  type ReflectEventMsg,
  type SessionConfiguredPayload,
  type TurnStartedPayload,
  type TurnCompletePayload,
  type TurnAbortedPayload,
  type TurnRewoundPayload,
  type AbortReasonPayload,
  type AgentMessagePayload,
  type AgentMessageDeltaPayload,
  type ThinkingDeltaPayload,
  type TokenCountPayload,
  type ToolCallBeginPayload,
  type ToolCallEndPayload,
  type ToolOutput,
  type ContentBlock,
  type ApprovalRequestPayload,
  type ApprovalKind,
  type AskUserQuestionPayload,
  type PermissionBubblePayload,
  type ContextCompactedPayload,
  type ErrorPayload,
  type StreamErrorPayload,
  type ConfigReloadedPayload,
  type RoutingPayload,
  type CollabStartedPayload,
  type CollabMessagePayload,
  type CollabFinishedPayload,
  type McpServerStartedPayload,
  type McpServerFailedPayload,
  type McpToolInvokedPayload,
  type LspServerStartedPayload,
  type LspServerFailedPayload,
  type PlanRequestPayload,
  type PlanReadyPayload,
  type PlanApprovedPayload,
  type PlanRejectedPayload,
  type PermissionModeChangedPayload,
} from './event';

// ----- Submission / Op -----
export {
  type OpType,
  type ReflectSubmissionOp,
  type ThreadSettingsOverrides,
  type W3cTraceContext,
} from './op';
export type { ReflectSubmission } from './submission';

// ----- UserInputItem -----
export type { UserInputItem } from './item';

// ----- Question / Answer -----
export type {
  Question,
  QuestionOption,
  Answer,
  AskUserAnswer,
} from './question';

// ----- AskUserInput -----
export type { AskUserInputPayload } from './ask_user_input';

// ----- Rollout / SessionInfo -----
export type { ReflectSessionInfo, ReflectRolloutRecord } from './rollout';

// ----- Shared enums -----
export type {
  PermissionMode,
  ApprovalPolicy,
  SandboxPolicy,
  RiskLevel,
  ReasoningEffort,
  ReviewDecision,
  PlanApprovalChoice,
} from './enums';

// ----- Usage -----
export type { TokenUsagePayload } from './usage';

// ----- Backward-compat aliases (used by older code that imports these
//       names) -----
export type { EventMsgType as ReflectEventType } from './event';
export type { OpType as ReflectSubmissionOpType } from './op';
export type { UserInputItem as ReflectUserInputItem } from './item';
