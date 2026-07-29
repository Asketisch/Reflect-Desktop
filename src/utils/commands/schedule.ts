/**
 * Schedule (cron) wrappers — Phase 1 item 2.
 *
 * Mirrors `src-tauri/src/commands/schedule.rs` (which wraps
 * `vendor/reflect-stream::cron::CronScheduler`). Payloads are the vendor types
 * serialized verbatim (snake_case, since `CronJobSpec` does not derive
 * `rename_all = "camelCase"`); only the `ScheduleStatus` envelope is camelCase
 * (matches the Rust `#[serde(rename_all = "camelCase")]`).
 *
 * Lifecycle: the scheduler starts empty; `install_agent_thread` injects the
 * real `AgentThread::submission_sender()` and spawns a 30s driver tick. Before
 * install completes, CRUD still works (the command reads the `RwLock`), but no
 * background firing happens. The driver fires a due job by injecting its
 * `prompt` as a `Submission::user_input` into the agent loop.
 *
 * Out of scope (next rounds): `run_now` (needs a vendor `pub async fn
 * run_now(&self)`), persistence (vendor scheduler is in-memory), tick-interval
 * config, one-shot `run_at`.
 */
import { invoke } from '../bridge';

// ── CronJobSpec (mirrors `reflect_stream::cron::CronJobSpec`, snake_case) ──

export interface ReflectCronJob {
  /** Stable uuid; used for update / remove. */
  id: string;
  /** 5-field cron expression (min hour dom month dow). */
  schedule: string;
  /** Prompt injected as `Submission::user_input` when the job fires. */
  prompt: string;
  /** Optional human-readable name; defaults to first 24 chars of prompt. */
  name?: string | null;
  /** Disabled jobs stay in the list but are not fired by the driver. */
  enabled: boolean;
  /** Creation time (UTC, ISO 8601). */
  created_at: string;
  /** Last fired time; `null` = never. */
  last_fired?: string | null;
  /** Computed next fire time; `null` if the expression has no solution. */
  next_fire?: string | null;
}

// ── ScheduleStatus (camelCase, mirrors `commands/schedule.rs::ScheduleStatus`) ─

export interface ReflectScheduleStatus {
  /** `idle` = no enabled job; `has_jobs` = at least one enabled job. */
  status: 'idle' | 'has_jobs';
  /** Human-readable status line (for status badge). */
  line: string;
  /** Total job count. */
  total: number;
  /** Enabled job count. */
  enabled: number;
}

// ── Commands ───────────────────────────────────────────────────────────

/** List all cron jobs (sorted by created_at ascending). */
export async function reflect_list_schedules(): Promise<ReflectCronJob[]> {
  return invoke<ReflectCronJob[]>('reflect_list_schedules');
}

/**
 * Register a new job. Validates the 5-field cron expression; throws on parse
 * error. `name` defaults to the first 24 chars of `prompt` when omitted.
 */
export async function reflect_add_schedule(args: {
  schedule: string;
  prompt: string;
  name?: string | null;
}): Promise<ReflectCronJob> {
  return invoke<ReflectCronJob>('reflect_add_schedule', {
    schedule: args.schedule,
    prompt: args.prompt,
    name: args.name ?? null,
  });
}

/**
 * Update a job (by id). Pass only the fields you want to change; changing
 * `schedule` recomputes `next_fire`. Throws if the id is not found or the
 * new schedule fails to parse.
 */
export async function reflect_update_schedule(args: {
  id: string;
  schedule?: string;
  prompt?: string;
  name?: string;
  enabled?: boolean;
}): Promise<ReflectCronJob> {
  return invoke<ReflectCronJob>('reflect_update_schedule', {
    id: args.id,
    schedule: args.schedule ?? null,
    prompt: args.prompt ?? null,
    name: args.name ?? null,
    enabled: args.enabled ?? null,
  });
}

/** Delete a job (by id). Returns `true` if deleted, `false` if id not found. */
export async function reflect_remove_schedule(id: string): Promise<boolean> {
  return invoke<boolean>('reflect_remove_schedule', { id });
}

/** Scheduler status snapshot (idle / has_jobs + counts). */
export async function reflect_get_schedule_status(): Promise<ReflectScheduleStatus> {
  return invoke<ReflectScheduleStatus>('reflect_get_schedule_status');
}
