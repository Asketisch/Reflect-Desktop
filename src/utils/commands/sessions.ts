/**
 * Session I/O — list / replay / rename / delete / export.
 *
 * `ReflectRolloutRecord` is intentionally `Record<string, unknown>` shaped
 * because Rust gives us a protocol-discriminated union; see
 * `src/types/protocol.ts` for the TS view.
 */
import { invoke } from '../bridge';
import type { ReflectRolloutRecord, ReflectSessionInfo } from '../types';

/** Session metadata returned by `reflect_list_sessions`. */
export type { ReflectSessionInfo } from '../types';

/** Result of `reflect_export_session_markdown`. */
export interface ReflectMarkdownExportResult {
  path: string;
  bytes: number;
}

/** Page through sessions. `limit` and `offset` are optional. */
export async function reflect_list_sessions(
  opts: { limit?: number; offset?: number } = {},
): Promise<ReflectSessionInfo[]> {
  return invoke<ReflectSessionInfo[]>('reflect_list_sessions', {
    limit: opts.limit ?? null,
    offset: opts.offset ?? null,
  });
}

/** Rename an existing session by id. */
export async function reflect_rename_session(id: string, new_name: string): Promise<void> {
  return invoke<void>('reflect_rename_session', { id, newName: new_name });
}

/** Delete an existing session by id. */
export async function reflect_delete_session(id: string): Promise<void> {
  return invoke<void>('reflect_delete_session', { id });
}

/** Replay a session: returns the rollout record stream the original events produced. */
export async function reflect_replay_session(id: string): Promise<ReflectRolloutRecord[]> {
  return invoke<ReflectRolloutRecord[]>('reflect_replay_session', { id });
}

/** Export session to JSON, returns absolute path. */
export async function reflect_export_session(id: string): Promise<string | null> {
  return invoke<string | null>('reflect_export_session', { id });
}

/** Export session to Markdown, returns the rendered file path. */
export async function reflect_export_session_markdown(
  id: string,
): Promise<ReflectMarkdownExportResult | null> {
  return invoke<ReflectMarkdownExportResult | null>('reflect_export_session_markdown', { id });
}
