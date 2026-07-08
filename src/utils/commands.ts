/**
 * 14 个 reflect_* command —— 每个 Op 一个 wrapper。
 *
 * CodexMonitor 同名: `src/services/tauri.ts::commands` 节
 *
 * 设计蓝图：`docs/PROTOCOL_BRIDGE.md` §2 + §6。
 */
import { invoke, listen } from './bridge';
import type { ReflectRolloutRecord, ReflectSessionInfo, ReviewDecision } from './types';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';

/** Health. */
export async function ping(): Promise<{ msg: string; version: string }> {
  return invoke<{ msg: string; version: string }>('ping');
}

/** Submission 入口。返回 submission.id。 */
export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return invoke<string>('reflect_submit', { submission: s });
}

/** Lifecycle / turn 控制。 */
export async function reflect_interrupt(): Promise<void> {
  return invoke<void>('reflect_interrupt');
}
export async function reflect_compact(): Promise<void> {
  return invoke<void>('reflect_compact');
}
export async function reflect_rewind(to_turn_id?: string): Promise<void> {
  return invoke<void>('reflect_rewind', { toTurnId: to_turn_id ?? null });
}
export async function reflect_shutdown(): Promise<void> {
  return invoke<void>('reflect_shutdown');
}

/** Approvals. */
export async function reflect_tool_approval(id: string, decision: ReviewDecision): Promise<void> {
  return invoke<void>('reflect_tool_approval', { id, decision });
}
export async function reflect_hook_approval(id: string, decision: ReviewDecision): Promise<void> {
  return invoke<void>('reflect_hook_approval', { id, decision });
}
export async function reflect_plan_approval(id: string, decision: ReviewDecision): Promise<void> {
  return invoke<void>('reflect_plan_approval', { id, decision });
}

/** Plan mode. */
export async function reflect_enter_plan_mode(task: string): Promise<void> {
  return invoke<void>('reflect_enter_plan_mode', { task });
}
export async function reflect_exit_plan_mode(): Promise<void> {
  return invoke<void>('reflect_exit_plan_mode');
}

/** Effort / permission. */
export async function reflect_set_effort(level: string): Promise<void> {
  return invoke<void>('reflect_set_effort', { level });
}
export async function reflect_set_permission_mode(mode: string): Promise<void> {
  return invoke<void>('reflect_set_permission_mode', { mode });
}
export async function reflect_cycle_permission_mode(): Promise<void> {
  return invoke<void>('reflect_cycle_permission_mode');
}

/** Ask user. */
export async function reflect_ask_user_question_response(id: string, answers: unknown): Promise<void> {
  return invoke<void>('reflect_ask_user_question_response', { id, answers });
}
export async function reflect_ask_user_input_response(id: string, text: string): Promise<void> {
  return invoke<void>('reflect_ask_user_input_response', { id, text });
}

/** Session I/O. */
export async function reflect_list_sessions(): Promise<ReflectSessionInfo[]> {
  return invoke<ReflectSessionInfo[]>('reflect_list_sessions');
}
export async function reflect_rename_session(id: string, new_name: string): Promise<void> {
  return invoke<void>('reflect_rename_session', { id, newName: new_name });
}
export async function reflect_delete_session(id: string): Promise<void> {
  return invoke<void>('reflect_delete_session', { id });
}
export async function reflect_replay_session(id: string): Promise<ReflectRolloutRecord[]> {
  return invoke<ReflectRolloutRecord[]>('reflect_replay_session', { id });
}

/** 订阅 reflect_event 通道（payload = ReflectEvent）。 */
export async function onReflectEvent(handler: (event: ReflectEvent) => void) {
  return listen<ReflectEvent>('reflect_event', (e) => handler(e.payload));
}