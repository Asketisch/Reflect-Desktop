/**
 * Reflect protocol — Rollout / SessionInfo.
 *
 * Mirrors `vendor/reflect-protocol/src/rollout.rs`. The `ReflectSessionInfo`
 * interface here is the *protocol* shape (snake_case) used by the Tauri
 * envelope — distinct from `src/utils/types.ts` `ReflectSessionInfo`,
 * which is the *backend* (Rust-crate) shape with kebab/extra fields
 * re-exported through `@/utils/commands`. Don't confuse the two.
 *
 * `ReflectRolloutRecord` wraps a `ReflectSubmission` or `ReflectEvent`
 * with `seq`, `kind`, `timestamp` for replay.
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

import type { ReflectSubmission } from './submission';
import type { ReflectEvent } from './event';

export interface ReflectSessionInfo {
  thread_id: string;
  name?: string;
  created_at: number;
  updated_at: number;
  message_count: number;
  token_total: number;
}

export interface ReflectRolloutRecord {
  seq: number;
  kind: 'submission' | 'event';
  timestamp: number;
  payload: ReflectSubmission | ReflectEvent;
}
