/**
 * Hook registry wrappers — list/toggle.
 */
import { invoke } from '../bridge';

export interface ReflectHookInfo {
  name: string;
  kind: string;
  enabled: boolean;
  config_summary: string;
}

/** List registered hooks. */
export async function reflect_list_hooks(): Promise<ReflectHookInfo[]> {
  return invoke<ReflectHookInfo[]>('reflect_list_hooks');
}

/** Enable or disable a hook by name. */
export async function reflect_toggle_hook(name: string, enabled: boolean): Promise<void> {
  return invoke<void>('reflect_toggle_hook', { name, enabled });
}
