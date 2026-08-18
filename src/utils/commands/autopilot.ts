/**
 * Autopilot IPC 包装。
 *
 * Phase 3 条目 10：自动任务调度。
 */
import { invoke } from '@/utils/bridge';

// ── 类型 ──

export interface ReflectAutopilotConfig {
  enabled: boolean;
  schedule: string;
  taskTemplate: string;
  agent: string | null;
  maxConcurrent: number;
}

export interface ReflectAutopilotRun {
  id: string;
  executedAtMs: number;
  taskId: string | null;
  status: 'success' | 'failed' | 'inProgress' | 'skipped';
  error: string | null;
}

// ── Wrappers ──

export function reflect_get_autopilot_config(): Promise<ReflectAutopilotConfig> {
  return invoke('reflect_get_autopilot_config');
}

export function reflect_update_autopilot_config(config: ReflectAutopilotConfig): Promise<void> {
  return invoke('reflect_update_autopilot_config', { config });
}

export function reflect_autopilot_history(): Promise<ReflectAutopilotRun[]> {
  return invoke('reflect_autopilot_history');
}