/**
 * Submission + lifecycle wrappers for the active agent turn.
 *
 * Lifecycle / Op wrappers each return the originating submission id so the
 * frontend can pair them with the live `reflect_event` stream.
 */
import { invoke } from '../bridge';
import type { ReflectSubmission } from '@/types/protocol';

/** Submission 入口。Returns the submission id. */
export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return invoke<string>('reflect_submit', { submission: s });
}

/** 中断当前 agent turn。 */
export async function reflect_interrupt(): Promise<void> {
  return invoke<void>('reflect_interrupt');
}

/** Compact current session history. */
export async function reflect_compact(): Promise<string> {
  return invoke<string>('reflect_compact');
}

/** Rewind to a previous turn (omit to drop the last user message). */
export async function reflect_rewind(to_turn_id?: string): Promise<string> {
  return invoke<string>('reflect_rewind', { toTurnId: to_turn_id ?? null });
}

/** Shut down the agent runtime. */
export async function reflect_shutdown(): Promise<string> {
  return invoke<string>('reflect_shutdown');
}
