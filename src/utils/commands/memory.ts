/**
 * MEMORY.md scope/key/value store wrappers.
 */
import { invoke } from '../bridge';

export interface ReflectMemoryEntry {
  scope: string;
  key: string;
  value: string;
}

/** List all memory entries across scopes. */
export async function reflect_list_memory(): Promise<ReflectMemoryEntry[]> {
  return invoke<ReflectMemoryEntry[]>('reflect_list_memory');
}

/** Add or overwrite a memory entry. */
export async function reflect_add_memory(scope: string, key: string, value: string): Promise<void> {
  return invoke<void>('reflect_add_memory', { scope, key, value });
}

/** Remove a memory entry by scope+key. */
export async function reflect_remove_memory(scope: string, key: string): Promise<void> {
  return invoke<void>('reflect_remove_memory', { scope, key });
}
