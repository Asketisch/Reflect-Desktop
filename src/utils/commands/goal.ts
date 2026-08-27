/**
 * Goal mode 封装 —— 进入/退出目标模式(Op::EnterGoalMode / Op::ExitGoalMode, v1.2 P1)。
 *
 * core 侧收到 EnterGoalMode 后构造 GoalController:每轮 turn 结束自校验
 * (LM judge + 可选 verify_command),未完成则 steering 续作,完成则退出。
 */
import { invoke } from '../bridge';

/** 进入目标模式;返回 submission id。 */
export async function reflect_enter_goal_mode(
  goal: string,
  verifyCommand?: string,
  tokenBudget?: number,
): Promise<string> {
  return invoke<string>('reflect_enter_goal_mode', { goal, verifyCommand, tokenBudget });
}

/** 退出目标模式,停止自动续作;返回 submission id。 */
export async function reflect_exit_goal_mode(): Promise<string> {
  return invoke<string>('reflect_exit_goal_mode');
}
