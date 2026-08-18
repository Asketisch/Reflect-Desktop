/**
 * 更新检查器封装。
 */
import { invoke } from '../bridge';

export interface ReflectUpdateInfo {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  release_url: string | null;
  release_notes: string | null;
  probed_at: number;
  error: string | null;
}

/**
 * 向上游 releases 探测更新。探测完全失败时返回 `null`。
 */
export async function reflect_check_update(): Promise<ReflectUpdateInfo | null> {
  return invoke<ReflectUpdateInfo | null>('reflect_check_update');
}
