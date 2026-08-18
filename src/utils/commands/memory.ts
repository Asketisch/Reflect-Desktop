/**
 * MEMORY.md scope/key/value 存储封装。
 */
import { invoke } from '../bridge';

export interface ReflectMemoryEntry {
  scope: string;
  key: string;
  value: string;
}

/** 列出所有 scope 下的 memory 条目。 */
export async function reflect_list_memory(): Promise<ReflectMemoryEntry[]> {
  return invoke<ReflectMemoryEntry[]>('reflect_list_memory');
}

/** 新增或覆盖一条 memory 条目。 */
export async function reflect_add_memory(scope: string, key: string, value: string): Promise<void> {
  return invoke<void>('reflect_add_memory', { scope, key, value });
}

/** 按 scope+key 删除一条 memory 条目。 */
export async function reflect_remove_memory(scope: string, key: string): Promise<void> {
  return invoke<void>('reflect_remove_memory', { scope, key });
}
