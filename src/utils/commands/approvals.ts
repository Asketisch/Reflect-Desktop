/**
 * Approval/dispatch wrappers — paired with the matching `ReviewRequest` event
 * streams from `onReflectEvent`.
 *
 * Tool/hook approval 接收 `ReviewDecision`(approve / deny);
 * plan approval 接收 `PlanApprovalChoice`(auto_mode / manual_approve / revise)——
 * 对齐后端 `reflect_protocol::PlanApprovalChoice`。
 */
import { invoke } from '../bridge';
import type { PlanApprovalChoice, ReviewDecision } from '../types';

/** Tool call approval. */
export async function reflect_tool_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_tool_approval', { id, decision });
}

/** Hook approval. */
export async function reflect_hook_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_hook_approval', { id, decision });
}

/** Plan approval —— 传入 `PlanApprovalChoice`(三选一)。 */
export async function reflect_plan_approval(id: string, choice: PlanApprovalChoice): Promise<string> {
  return invoke<string>('reflect_plan_approval', { id, choice });
}
