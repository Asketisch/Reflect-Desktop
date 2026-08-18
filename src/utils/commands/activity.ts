/**
 * 活动时间线 IPC 包装（Phase 3 条目 9）。
 *
 * 薄包装 `reflect_list_activity` / `reflect_search_activity` /
 * `reflect_clear_activity` / `reflect_activity_count` 四个命令。
 * 与后端 `reflect_app_core::activity::{ActivityEvent, ActivityFilter, Actor}`
 * camelCase 类型对齐。
 */
import { invoke } from '@/utils/bridge';

// ── Actor (与 app-core/src/actor.rs 对齐) ──────────────────────────

export type ReflectActorType = 'human' | 'agent' | 'system';

export type ReflectActorKind = 'user' | 'lead' | 'member' | 'system';

export interface ReflectActor {
  actorType: ReflectActorType;
  actorId: string;
  kind: ReflectActorKind;
  displayName?: string;
  teamName?: string;
}

// ── Activity kinds / levels ────────────────────────────────────────

export type ReflectActivityKind =
  | 'turn_started'
  | 'turn_complete'
  | 'turn_aborted'
  | 'agent_message'
  | 'tool_call_begin'
  | 'tool_call_end'
  | 'approval_request'
  | 'plan_request'
  | 'plan_approved'
  | 'plan_rejected'
  | 'task_created'
  | 'task_completed'
  | 'task_updated'
  | 'task_claimed'
  | 'team_created'
  | 'team_deleted'
  | 'mention'
  | 'error'
  | 'session_configured'
  | 'permission_mode_changed'
  | 'mcp_server'
  | 'lsp_server'
  | 'context_compacted'
  | 'other';

export type ReflectActivityLevel = 'info' | 'warn' | 'error';

export interface ReflectActivityEvent {
  id: string;
  tsMs: number;
  kind: ReflectActivityKind;
  actor: ReflectActor;
  summary: string;
  taskId?: number;
  teamName?: string;
  level: ReflectActivityLevel;
}

export interface ReflectActivityFilter {
  kind?: ReflectActivityKind;
  level?: ReflectActivityLevel;
  actorId?: string;
  teamName?: string;
  sinceMs?: number;
  limit?: number;
}

// ── Wrappers ───────────────────────────────────────────────────────

export function reflect_list_activity(
  filter?: ReflectActivityFilter,
): Promise<ReflectActivityEvent[]> {
  return invoke('reflect_list_activity', { filter: filter ?? null });
}

export function reflect_search_activity(query: string): Promise<ReflectActivityEvent[]> {
  return invoke('reflect_search_activity', { query });
}

export function reflect_clear_activity(): Promise<void> {
  return invoke('reflect_clear_activity');
}

export function reflect_activity_count(): Promise<number> {
  return invoke('reflect_activity_count');
}

/**
 * 从文本里提取 `@<token>` 形式的 mention。
 *
 * token 允许 `@team-lead@rocket`(嵌套 @)与 `@user` / `@system`。
 * 用于 composer mention 高亮 + Activity mentions 子视图过滤。
 */
export function extractMentions(text: string): string[] {
  const re = /@([a-z0-9_@-]+)/gi;
  const out = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    out.add(m[1]);
  }
  return [...out];
}
