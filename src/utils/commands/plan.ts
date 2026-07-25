/**
 * Plan mode wrappers — enter/exit a plan-only session.
 */
import { invoke } from '../bridge';

/** Enter plan mode for a given task; returns submission id. */
export async function reflect_enter_plan_mode(task: string): Promise<string> {
  return invoke<string>('reflect_enter_plan_mode', { task });
}

/** Exit plan mode; returns submission id. */
export async function reflect_exit_plan_mode(): Promise<string> {
  return invoke<string>('reflect_exit_plan_mode');
}
