/**
 * 审批 allowlist 封装 —— 按前缀匹配的"始终允许"规则。
 */
import { invoke } from '../bridge';

export interface ReflectAllowlist {
  prefixes: string[];
}

/** 加载持久化的 allowlist(无规则时返回 `null`)。 */
export async function reflect_load_allowlist(): Promise<ReflectAllowlist | null> {
  return invoke<ReflectAllowlist | null>('reflect_load_allowlist');
}

/** 整体保存 allowlist。 */
export async function reflect_save_allowlist(list: ReflectAllowlist): Promise<void> {
  return invoke<void>('reflect_save_allowlist', { list });
}

/** 检查给定命令前缀是否已被加入 allowlist。 */
export async function reflect_check_allowlist(prefix: string): Promise<boolean | null> {
  return invoke<boolean | null>('reflect_check_allowlist', { prefix });
}
