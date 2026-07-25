/**
 * Health / probe commands.
 *
 * Wraps the standalone `ping` command used to probe the Tauri backend
 * during startup and from `bridge.test.ts`.
 */
import { invoke } from '../bridge';

/** Probe the Tauri backend; returns the protocol/agent version banner. */
export async function ping(): Promise<{ msg: string; version: string }> {
  return invoke<{ msg: string; version: string }>('ping');
}
