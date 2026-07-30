/**
 * src/protocol/submissions.ts — Submission constructors (B1-03).
 *
 * Single source of truth for all `Submission` payloads the frontend
 * constructs. Each function:
 *   1. Generates a fresh `id` (or accepts an explicit one).
 *   2. Builds the strongly-typed `op` payload.
 *   3. Returns a complete `ReflectSubmission` ready for `reflect_submit`.
 *
 * Why this exists:
 *   Before, callers (`agentStore.submit`, modals, slash engine) hand-built
 *   `Submission` objects inline, leading to inconsistent `id` generation
 *   and occasional missing fields. Centralizing here:
 *   - Guarantees every Submission has a unique id.
 *   - Gives one place to add `client_user_message_id` / `trace` later.
 *   - Lets the slash engine (B4) call the same constructors as the store.
 *
 * Naming convention: every exported builder is a noun + verb matching
 * the `Op` variant. e.g. `Op::ToolApproval` → `toolApproval(...)`.
 */

import {
  EVENT_ID_NONE,
  type AskUserAnswer,
  type PermissionMode,
  type PlanApprovalChoice,
  type ReflectSubmission,
  type ReflectSubmissionOp,
  type ReasoningEffort,
  type ReviewDecision,
  type UserInputItem,
} from '@/types/protocol';

// ====== ID generation ======

let __submissionCounter = 0;

/** Generate a unique submission id. Falls back to timestamp+counter if
 * `crypto.randomUUID` is unavailable (jsdom test env).
 */
export function newSubmissionId(prefix = 'sub'): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}-${crypto.randomUUID()}`;
  }
  __submissionCounter += 1;
  return `${prefix}-${Date.now()}-${__submissionCounter}`;
}

/** Reset the counter (test-only). */
export function __resetSubmissionIdCounter(): void {
  __submissionCounter = 0;
}

// ====== Submission builder ======

/** Internal: build a Submission with auto id. */
function build(op: ReflectSubmissionOp, id = newSubmissionId()): ReflectSubmission {
  return { id, op };
}

// ====== Per-Op constructors ======

/**
 * `Op::UserInput` — submit one or more text/image/skill items to the agent.
 * Most callers should use `userInputText` for the simple case.
 */
export function userInput(
  items: UserInputItem[],
  threadSettings?: ReflectSubmission['op'] extends { type: 'user_input'; thread_settings?: infer T } ? T : never,
  id = newSubmissionId(),
): ReflectSubmission {
  const op: ReflectSubmissionOp = { type: 'user_input', items };
  if (threadSettings) (op as { thread_settings?: unknown }).thread_settings = threadSettings;
  return { id, op };
}

/** Convenience: submit a single text message. */
export function userInputText(text: string, id = newSubmissionId()): ReflectSubmission {
  return userInput([{ type: 'text', text }], undefined, id);
}

/** Submit a local image (file path) plus optional caption. */
export function userInputImage(path: string, text?: string, id = newSubmissionId()): ReflectSubmission {
  const items: UserInputItem[] = [{ type: 'local_image', path }];
  if (text) items.push({ type: 'text', text });
  return userInput(items, undefined, id);
}

/** Activate a skill (B5: skill mention picker). */
export function userInputSkill(name: string, args?: unknown, id = newSubmissionId()): ReflectSubmission {
  return userInput([{ type: 'skill', name, args }], undefined, id);
}

/** `Op::Compact` — trigger context compaction. */
export function compact(id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'compact' }, id);
}

/** `Op::Interrupt` — cancel the current turn. */
export function interrupt(id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'interrupt' }, id);
}

/** `Op::Rewind` — rewind conversation to a previous turn (or to last user turn if `to_turn_id` is undefined). */
export function rewind(toTurnId?: string | null, id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'rewind', to_turn_id: toTurnId ?? null }, id);
}

/** `Op::Shutdown` — gracefully shut down the agent. */
export function shutdown(id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'shutdown' }, id);
}

/** `Op::ToolApproval` — approve / deny / approve-for-session a tool call. */
export function toolApproval(
  id_: string,
  decision: ReviewDecision,
  id = newSubmissionId(),
): ReflectSubmission {
  return build({ type: 'tool_approval', id: id_, decision }, id);
}

/** `Op::HookApproval` — approve / deny a hook decision. */
export function hookApproval(
  id_: string,
  decision: ReviewDecision,
  id = newSubmissionId(),
): ReflectSubmission {
  return build({ type: 'hook_approval', id: id_, decision }, id);
}

/** `Op::PlanApproval` — plan 审批三选一(auto_mode / manual_approve / revise)。 */
export function planApproval(
  id_: string,
  choice: PlanApprovalChoice,
  id = newSubmissionId(),
): ReflectSubmission {
  return build({ type: 'plan_approval', id: id_, choice }, id);
}

/** `Op::EnterPlanMode` — request entry to plan mode. */
export function enterPlanMode(task: string, id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'enter_plan_mode', task }, id);
}

/** `Op::ExitPlanMode` — request exit from plan mode. */
export function exitPlanMode(id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'exit_plan_mode' }, id);
}

/** `Op::SetEffort` — set reasoning effort. */
export function setEffort(effort: ReasoningEffort, id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'set_effort', effort }, id);
}

/** `Op::SetPermissionMode` — switch to a specific permission mode. */
export function setPermissionMode(mode: PermissionMode, id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'set_permission_mode', mode }, id);
}

/** `Op::CyclePermissionMode` — cycle to the next permission mode in UI cycle. */
export function cyclePermissionMode(id = newSubmissionId()): ReflectSubmission {
  return build({ type: 'cycle_permission_mode' }, id);
}

/** `Op::AskUserQuestionResponse` — answer a structured question. */
export function askUserQuestionResponse(
  id_: string,
  answers: AskUserAnswer,
  id = newSubmissionId(),
): ReflectSubmission {
  return build({ type: 'ask_user_question_response', id: id_, answers }, id);
}

/** `Op::AskUserInputResponse` — answer a free-text input question. */
export function askUserInputResponse(
  id_: string,
  text: string,
  id = newSubmissionId(),
): ReflectSubmission {
  return build({ type: 'ask_user_input_response', id: id_, text }, id);
}

// ====== Re-export event constant ======
export { EVENT_ID_NONE };
