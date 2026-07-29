/**
 * Autopilot IPC wrappers.
 *
 * Phase 3 item 10: automatic task scheduling.
 */
import { invoke } from '@/utils/bridge';

// ── Types ──

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