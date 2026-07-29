/**
 * Remote mode wrappers — Phase 2 item 2 (Tailscale + iOS daemon).
 *
 * Mirrors `src-tauri/src/commands/remote.rs` (which wraps
 * `reflect-app-core::tailscale` + `state::RemoteConfig`).
 *
 * Out of scope (next rounds):
 *   - Independent TCP JSON-RPC daemon binary (`reflect_daemon`).
 *   - Transport driver that maintains a live connection state.
 *   - `reflect_tailscale_daemon_start / stop / status` are placeholder
 *     commands; they return `"not implemented yet"` so the frontend can
 *     surface a friendly message instead of pretending to start a daemon
 *     that doesn't exist yet.
 */
import { invoke } from '../bridge';

// ── Types (mirror `state::RemoteConfig` / `state::RemoteStatus` / app-core tailscale) ──

export interface ReflectRemoteConfigSnapshot {
  host: string;
  port: number;
  auth_token: string | null;
  auto_connect: boolean;
  endpoint: string;
  is_ready: boolean;
}

export interface ReflectRemoteStatus {
  state: 'connected' | 'disconnected' | 'error' | string;
  message: string | null;
  endpoint: string | null;
  sinceMs: number | null;
}

/** Tailscale status (camelCase, mirrors `app_core::tailscale::TailscaleStatus`). */
export interface ReflectTailscaleStatus {
  installed: boolean;
  running: boolean;
  version: string | null;
  dns_name: string | null;
  host_name: string | null;
  tailnet_name: string | null;
  ipv4: string[];
  ipv6: string[];
  suggested_remote_host: string | null;
  message: string | null;
}

// ── Commands ───────────────────────────────────────────────────────────

/** Read the current remote config (host / port / auth_token / auto_connect). */
export async function reflect_get_remote_config(): Promise<ReflectRemoteConfigSnapshot> {
  return invoke<ReflectRemoteConfigSnapshot>('reflect_get_remote_config');
}

/** Overwrite the remote config and return the updated snapshot. */
export async function reflect_update_remote_config(args: {
  host: string;
  port?: number;
  auth_token?: string | null;
  auto_connect?: boolean;
}): Promise<ReflectRemoteConfigSnapshot> {
  return invoke<ReflectRemoteConfigSnapshot>('reflect_update_remote_config', {
    host: args.host,
    port: args.port ?? null,
    auth_token: args.auth_token ?? null,
    auto_connect: args.auto_connect ?? null,
  });
}

/** Read the runtime transport state (always `disconnected` until the driver is wired). */
export async function reflect_get_remote_status(): Promise<ReflectRemoteStatus> {
  return invoke<ReflectRemoteStatus>('reflect_get_remote_status');
}

/** Probe the local Tailscale daemon. Returns a degraded status on any failure. */
export async function reflect_tailscale_status(): Promise<ReflectTailscaleStatus> {
  return invoke<ReflectTailscaleStatus>('reflect_tailscale_status');
}

/** Hint string for the iOS setup card (the command the user runs on the desktop). */
export async function reflect_tailscale_daemon_command_preview(): Promise<string> {
  return invoke<string>('reflect_tailscale_daemon_command_preview');
}

/** Placeholder: start the desktop daemon. Currently returns a `not implemented yet` note. */
export async function reflect_tailscale_daemon_start(): Promise<string> {
  return invoke<string>('reflect_tailscale_daemon_start');
}

/** Placeholder: stop the desktop daemon. */
export async function reflect_tailscale_daemon_stop(): Promise<string> {
  return invoke<string>('reflect_tailscale_daemon_stop');
}

/** Placeholder: query the desktop daemon's running state. */
export async function reflect_tailscale_daemon_status(): Promise<string> {
  return invoke<string>('reflect_tailscale_daemon_status');
}