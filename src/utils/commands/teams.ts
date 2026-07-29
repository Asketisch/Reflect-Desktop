/**
 * Team management wrappers — Phase 1 multi-agent command surface.
 *
 * Mirrors the Team half of `src-tauri/src/commands/tasks.rs` (which wraps
 * `vendor/reflect-task::TaskManager`). Task wrappers live alongside in
 * `./tasks.ts`. Team payloads are the vendor types serialized verbatim
 * (snake_case, since `TeamFile` does not derive `rename_all = "camelCase"`).
 *
 * Storage reuse: data lives at `~/.reflect/teams/<name>.json`, shared with
 * the TUI/CLI.
 */
import { invoke } from '../bridge';

// ── Team member (mirrors `reflect_task::TeamMemberSpec`, snake_case) ────

export interface ReflectTeamMember {
  /** `<role>@<team>`; lead is always `team-lead@<team>`. */
  agent_id: string;
  /** Human-readable label (e.g. `"team-lead"` / `"architect"`). */
  name: string;
  /** kebab-case role. */
  role: string;
  model?: string | null;
  system_prompt: string;
  /** Subset of parent tool names this member can invoke; empty = text-only. */
  allowed_tools: string[];
  color?: string | null;
  joined_at: number | string;
  session_id?: string | null;
  /** Channels this member subscribes to (M9; Phase 2 leaves empty). */
  subscriptions: string[];
}

// ── Team (mirrors `reflect_task::TeamFile`, snake_case) ────────────────

export interface ReflectTeam {
  name: string;
  description?: string | null;
  /** Always `team-lead@<name>`; validated by `TaskManager::upsert_team`. */
  lead_agent_id: string;
  lead_session_id?: string | null;
  members: ReflectTeamMember[];
  created_at: number | string;
}

// ── Commands ───────────────────────────────────────────────────────────

/** List all teams, sorted by name. */
export async function reflect_list_teams(): Promise<ReflectTeam[]> {
  return invoke<ReflectTeam[]>('reflect_list_teams');
}

/**
 * Idempotent upsert. Validates: `name` is legal (`[a-z0-9_-]+`, 1..=64 chars)
 * and `lead_agent_id === "team-lead@<name>"`.
 */
export async function reflect_upsert_team(team: ReflectTeam): Promise<void> {
  return invoke<void>('reflect_upsert_team', { team });
}

/** Read a single team; throws on not-found. */
export async function reflect_get_team(name: string): Promise<ReflectTeam> {
  return invoke<ReflectTeam>('reflect_get_team', { name });
}

/** Physically delete a team file (does NOT cascade-delete its tasks). */
export async function reflect_delete_team(name: string): Promise<void> {
  return invoke<void>('reflect_delete_team', { name });
}
