/**
 * Reflect protocol — Submission Op discriminated union.
 *
 * Mirrors `vendor/reflect-protocol/src/op.rs` `Op`. Each variant is
 * tagged with a `type` discriminator matching the Rust
 * `#[serde(rename_all = "snake_case")]` enum. The `OpType` string union
 * below is the primitive list of those discriminators.
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

import type { AskUserAnswer } from './question';
import type { UserInputItem } from './item';
import type {
  ApprovalPolicy,
  ReasoningEffort,
  ReviewDecision,
  SandboxPolicy,
  PermissionMode,
} from './enums';

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
