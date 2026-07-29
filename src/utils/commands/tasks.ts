/**
 * Task management wrappers — Phase 1 multi-agent command surface.
 *
 * Mirrors the Task half of `src-tauri/src/commands/tasks.rs` (which wraps
 * `vendor/reflect-task::TaskManager`). Team wrappers live alongside in
 * `./teams.ts`. Task payloads are the vendor types serialized verbatim
 * (snake_case, since `Task` does not derive `rename_all = "camelCase"`);
 * only the `TaskUpdateResult` envelope is camelCase (see `commands/tasks.rs`).
 *
 * Storage reuse: data lives at `~/.reflect/tasks/<list>/<id>.json`, shared
 * with the TUI/CLI.
 */
import { invoke } from '../bridge';

// ── Task status (snake_case, mirrors `reflect_task::TaskStatus`) ───────

export type ReflectTaskStatus = 'pending' | 'in_progress' | 'completed' | 'deleted';

// ── Task (mirrors `reflect_task::Task`, snake_case) ────────────────────

export interface ReflectTask {
  id: number;
  list_id: string;
  subject: string;
  description: string;
  active_form?: string | null;
  owner?: string | null;
  status: ReflectTaskStatus;
  /** Downstream task ids this task blocks. */
  blocks: number[];
  /** Upstream task ids that must complete before this one can start. */
  blocked_by: number[];
  metadata: Record<string, unknown>;
  output_path?: string | null;
  /** Worker id that atomically claimed the task (`<role>@<team>` or session id). */
  claimed_by?: string | null;
  claimed_at?: number | string | null;
  created_at: number | string;
  updated_at: number | string;
}

// ── TaskPatch (mirrors `reflect_task::TaskPatch`) ──────────────────────
//
// All fields optional; pass only what you want to change.
// `Some("")` clears a string field; `undefined` (omitted) leaves it alone.
// Null inside `active_form` / `owner` / `claimed_by` / `claimed_at` means
// "clear", matching Rust `Option<Option<T>>` tri-state semantics.

export interface ReflectTaskPatch {
  subject?: string;
  description?: string;
  active_form?: string | null;
  owner?: string | null;
  status?: ReflectTaskStatus;
  metadata?: Record<string, unknown>;
  /** Append downstream task ids this task blocks (unique). */
  add_blocks?: number[];
  /** Append upstream task ids that block this task (unique). */
  add_blocked_by?: number[];
  /** `string` = set, `null` = clear, `undefined` = leave unchanged. */
  claimed_by?: string | null;
  claimed_at?: number | string | null;
}

// ── TaskUpdateResult (camelCase, mirrors `commands/tasks.rs::TaskUpdateResult`) ─

export interface ReflectTaskStatusChange {
  from: string;
  to: string;
}

export interface ReflectTaskUpdateResult {
  /** The full task object after the update. */
  task: ReflectTask;
  /** Field names that actually changed (e.g. `["status", "subject"]`). */
  updatedFields: string[];
  /** Present only when `status` actually changed. */
  statusChange?: ReflectTaskStatusChange;
}

// ── Commands ───────────────────────────────────────────────────────────

/** List tasks under a list (worker view = session id; team view = team name). */
export async function reflect_list_tasks(list: string): Promise<ReflectTask[]> {
  return invoke<ReflectTask[]>('reflect_list_tasks', { list });
}

/** Create a task. `metadata` defaults to `{}`. */
export async function reflect_create_task(args: {
  list: string;
  subject: string;
  description: string;
  active_form?: string | null;
  owner?: string | null;
  metadata?: Record<string, unknown>;
}): Promise<ReflectTask> {
  return invoke<ReflectTask>('reflect_create_task', {
    list: args.list,
    subject: args.subject,
    description: args.description,
    active_form: args.active_form ?? null,
    owner: args.owner ?? null,
    metadata: args.metadata ?? null,
  });
}

/** Read a single task (excludes soft-deleted). */
export async function reflect_get_task(list: string, id: number): Promise<ReflectTask> {
  return invoke<ReflectTask>('reflect_get_task', { list, id });
}

/**
 * Update a task. Pass only the fields you want to change.
 * `status = "completed"` fires `TaskCompleted` hook (when a hook engine is
 * wired); other changes fire `TaskUpdated`.
 */
export async function reflect_update_task(
  list: string,
  id: number,
  patch: ReflectTaskPatch,
): Promise<ReflectTaskUpdateResult> {
  return invoke<ReflectTaskUpdateResult>('reflect_update_task', { list, id, patch });
}

/**
 * Atomically claim the next available task in a list: the first task that is
 * `pending` with empty `blocked_by`. Returns `null` when nothing is claimable.
 * Concurrent workers are serialized per-list; no duplicate claims.
 */
export async function reflect_claim_task(
  list: string,
  claimer: string,
): Promise<ReflectTask | null> {
  return invoke<ReflectTask | null>('reflect_claim_task', { list, claimer });
}

/** Physically delete a task file (distinct from `update_task({ status: "deleted" })` soft-delete). */
export async function reflect_delete_task(list: string, id: number): Promise<void> {
  return invoke<void>('reflect_delete_task', { list, id });
}
