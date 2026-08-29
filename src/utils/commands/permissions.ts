/**
 * Effort + permission-mode 封装。控制运行中 agent 循环的推理强度
 * 与全局权限门控。
 */
import { invoke } from '../bridge';

/** 设置推理强度等级(provider 定义字符串)。 */
export async function reflect_set_effort(level: string): Promise<string> {
  return invoke<string>('reflect_set_effort', { level });
}

/** 读回当前 reasoning effort("low"|"medium"|"high";pre-install 为 "low")。 */
export async function reflect_get_effort(): Promise<string> {
  return invoke<string>('reflect_get_effort');
}

/** 直接设置 permission mode。 */
export async function reflect_set_permission_mode(mode: string): Promise<string> {
  return invoke<string>('reflect_set_permission_mode', { mode });
}

/** 在可用的 permission mode 之间循环切换。 */
export async function reflect_cycle_permission_mode(): Promise<string> {
  return invoke<string>('reflect_cycle_permission_mode');
}
