/**
 * Approval allowlist wrappers — per-prefix "Always allow" rules.
 */
import { invoke } from '../bridge';

export interface ReflectAllowlist {
  prefixes: string[];
}

/** Load the persisted allowlist (returns `null` when no rules exist). */
export async function reflect_load_allowlist(): Promise<ReflectAllowlist | null> {
  return invoke<ReflectAllowlist | null>('reflect_load_allowlist');
}

/** Persist the allowlist wholesale. */
export async function reflect_save_allowlist(list: ReflectAllowlist): Promise<void> {
  return invoke<void>('reflect_save_allowlist', { list });
}

/** Check whether a given command prefix is allowlisted. */
export async function reflect_check_allowlist(prefix: string): Promise<boolean | null> {
  return invoke<boolean | null>('reflect_check_allowlist', { prefix });
}
