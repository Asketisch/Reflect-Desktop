/**
 * Hook 注册表封装 —— 列表 / 启停。
 */
import { invoke } from '../bridge';

export interface ReflectHookInfo {
  name: string;
  kind: string;
  enabled: boolean;
  config_summary: string;
}

/** 列出已注册的 hooks。 */
export async function reflect_list_hooks(): Promise<ReflectHookInfo[]> {
  return invoke<ReflectHookInfo[]>('reflect_list_hooks');
}

/** 按名称启用或禁用某个 hook。 */
export async function reflect_toggle_hook(name: string, enabled: boolean): Promise<void> {
  return invoke<void>('reflect_toggle_hook', { name, enabled });
}
