/**
 * agentStore 派生选择器 —— 供 Composer / 通知 / 队列等消费方复用，
 * 避免「turn 是否在跑」的判断散落各处。
 */
import type { AgentState } from './types';

/** 当前会话是否存在运行中的 turn（status === 'streaming'）。 */
export function selectIsTurnRunning(state: Pick<AgentState, 'turns'>): boolean {
  return state.turns.some((turn) => turn.status === 'streaming');
}

/**
 * 是否存在待用户处理的交互（审批 / 提问 / 自由输入 / plan 审批）。
 * 队列排空在此时暂停 —— 消息不应插在「agent 等用户决定」的中间。
 */
export function selectHasPendingInteraction(state: Pick<AgentState, 'pendingApprovals' | 'pendingQuestions' | 'pendingAskUser' | 'pendingPlan'>): boolean {
  return (
    state.pendingApprovals.length > 0 ||
    state.pendingQuestions.length > 0 ||
    state.pendingAskUser.length > 0 ||
    state.pendingPlan !== null
  );
}
