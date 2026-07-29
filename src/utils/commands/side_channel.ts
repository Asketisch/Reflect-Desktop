/**
 * Side-channel wrappers — Phase 2 item 1.
 *
 * Mirrors `src-tauri/src/commands/side_channel.rs` (which wraps
 * `reflect-app-core::side_channel::SideChannelRegistry`). The registry holds
 * one entry per side-channel with an independent `CancelToken`; main agent
 * `Cmd+C` does NOT cancel side-channels. Per-side-channel status transitions
 * (`Started` / `Done` / `Cancelled` / `Error`) are streamed to the frontend via
 * the existing `reflect_event` channel (events tagged
 * `kind: "side_channel_*"`); this wrapper file exposes the ad-hoc CRUD for
 * page reloads and slash commands.
 *
 * Out of scope (next rounds): a driver task that actually runs the side-channel
 * (wires its prompt as `Submission::user_input` into the agent loop and updates
 * the registry entry on completion). The registry + commands are in place
 * so the driver can be added without changing the IPC surface.
 */
import { invoke } from '../bridge';

// ── Types (mirrors `reflect_app_core::side_channel::{Status, Info}`, camelCase) ──

export type ReflectSideChannelStatus = 'running' | 'done' | 'cancelled' | 'error';

export interface ReflectSideChannelInfo {
  /** Stable id (`side-<8hex>`). */
  id: string;
  /** Agent definition name. */
  agentName: string;
  /** Initial prompt. */
  prompt: string;
  /** UNIX epoch milliseconds when the side-channel was registered. */
  startedAtMs: number;
  /** Current status. */
  status: ReflectSideChannelStatus;
  /** Elapsed milliseconds; `null` while still running. */
  durationMs: number | null;
}

export interface ReflectStartSideChannelResult {
  id: string;
  agentName: string;
  prompt: string;
  startedAtMs: number;
}

// ── Commands ───────────────────────────────────────────────────────────

/**
 * Start a new side-channel. Returns the assigned id + snapshot fields.
 * Emits a `kind: "side_channel_started"` event on the registry's broadcast
 * channel (forwarded as part of the existing Tauri `reflect_event` stream
 * by the install-time event forwarder).
 */
export async function reflect_start_side_channel(args: {
  agent_name: string;
  prompt: string;
}): Promise<ReflectStartSideChannelResult> {
  return invoke<ReflectStartSideChannelResult>('reflect_start_side_channel', {
    agent_name: args.agent_name,
    prompt: args.prompt,
  });
}

/**
 * Cancel a running side-channel by id. Returns `true` if cancellation took
 * effect, `false` if not found / already terminal.
 */
export async function reflect_cancel_side_channel(id: string): Promise<boolean> {
  return invoke<boolean>('reflect_cancel_side_channel', { id });
}

/** List all current side-channels (snapshot). The frontend relies on the
 * event stream for live updates; this is for ad-hoc reads (e.g. page reload). */
export async function reflect_list_side_channels(): Promise<ReflectSideChannelInfo[]> {
  return invoke<ReflectSideChannelInfo[]>('reflect_list_side_channels');
}

/** Single side-channel snapshot. Throws if id not found. */
export async function reflect_get_side_channel(id: string): Promise<ReflectSideChannelInfo> {
  return invoke<ReflectSideChannelInfo>('reflect_get_side_channel', { id });
}
