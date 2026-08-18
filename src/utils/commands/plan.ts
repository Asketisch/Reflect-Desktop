/**
 * Plan mode 封装 —— 进入/退出 plan-only 会话。
 */
import { invoke } from '../bridge';

/** 为指定任务进入 plan mode;返回 submission id。 */
export async function reflect_enter_plan_mode(task: string): Promise<string> {
  return invoke<string>('reflect_enter_plan_mode', { task });
}

/** 退出 plan mode;返回 submission id。 */
export async function reflect_exit_plan_mode(): Promise<string> {
  return invoke<string>('reflect_exit_plan_mode');
}
