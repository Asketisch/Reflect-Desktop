/**
 * Update checker wrapper.
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
 * Probe the upstream releases for an update. Returns `null` if the probe
 * couldn't complete at all.
 */
export async function reflect_check_update(): Promise<ReflectUpdateInfo | null> {
  return invoke<ReflectUpdateInfo | null>('reflect_check_update');
}
