/**
 * Effort + permission-mode wrappers. Controls the live agent loop's reasoning
 * effort and the global permission gate.
 */
import { invoke } from '../bridge';

/** Set reasoning effort level (provider-defined string). */
export async function reflect_set_effort(level: string): Promise<string> {
  return invoke<string>('reflect_set_effort', { level });
}

/** Set the permission mode directly. */
export async function reflect_set_permission_mode(mode: string): Promise<string> {
  return invoke<string>('reflect_set_permission_mode', { mode });
}

/** Cycle through available permission modes. */
export async function reflect_cycle_permission_mode(): Promise<string> {
  return invoke<string>('reflect_cycle_permission_mode');
}
