/**
 * Approval/dispatch wrappers — paired with the matching `ReviewRequest` event
 * streams from `onReflectEvent`.
 *
 * Each approval takes the approval id from the originating event and the
 * ReviewDecision (matches `reflect_protocol::ReviewDecision`).
 */
import { invoke } from '../bridge';
import type { ReviewDecision } from '../types';

/** Tool call approval. */
export async function reflect_tool_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_tool_approval', { id, decision });
}

/** Hook approval. */
export async function reflect_hook_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_hook_approval', { id, decision });
}

/** Plan approval. */
export async function reflect_plan_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_plan_approval', { id, decision });
}
