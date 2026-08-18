/**
 * 活动 agent 转数的 Submission + 生命周期包装。
 *
 * 生命周期 / Op 包装各自返回发起方的 submission id，
 * 便于前端将其与实时 `reflect_event` 流关联。
 */
import { invoke } from '../bridge';
import type { ReflectSubmission } from '@/types/protocol';

/** Submission 入口。返回 submission id。 */
export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return invoke<string>('reflect_submit', { submission: s });
}

/** 中断当前 agent turn。 */
export async function reflect_interrupt(): Promise<void> {
  return invoke<void>('reflect_interrupt');
}

/** 压缩当前会话历史。 */
export async function reflect_compact(): Promise<string> {
  return invoke<string>('reflect_compact');
}

/** 回退到上一个回合（省略参数则丢弃最后一条用户消息）。 */
export async function reflect_rewind(to_turn_id?: string): Promise<string> {
  return invoke<string>('reflect_rewind', { toTurnId: to_turn_id ?? null });
}

/** 关闭 agent 运行时。 */
export async function reflect_shutdown(): Promise<string> {
  return invoke<string>('reflect_shutdown');
}
