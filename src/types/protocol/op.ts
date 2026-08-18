/**
 * Reflect protocol —— Submission Op 判别联合。
 *
 * 对应 `reflect-agent/crates/protocol/reflect-protocol/src/op.rs` 中的 `Op`。每个变体使用
 * `type` discriminator 标记,与 Rust 的
 * `#[serde(rename_all = "snake_case")]` 枚举对齐。下方的 `OpType` 字符串
 * 联合即为这些 discriminator 的原始列表。
 *
 * 顶层同步警告见 `./index.ts`。
 */

import type { AskUserAnswer } from './question';
import type { UserInputItem } from './item';
import type {
  ApprovalPolicy,
  ReasoningEffort,
  ReviewDecision,
  PlanApprovalChoice,
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
  | { type: 'plan_approval'; id: string; choice: PlanApprovalChoice }
  | { type: 'set_effort'; effort: ReasoningEffort }
  | { type: 'ask_user_question_response'; id: string; answers: AskUserAnswer }
  | { type: 'ask_user_input_response'; id: string; text: string }
  | { type: 'set_permission_mode'; mode: PermissionMode }
  | { type: 'cycle_permission_mode' };
