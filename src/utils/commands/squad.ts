/**
 * Squad + Leader 委派 IPC 封装(Phase 3 第 11 项)。
 *
 * 薄封装 6 个 squad 命令。与后端 `reflect_app_core::squad::SquadSpec`
 * camelCase 类型对齐;Task 类型复用 `./tasks` 中的 `ReflectTask`。
 */
import { invoke } from '@/utils/bridge';
import type { ReflectActor } from './activity';
import type { ReflectTask } from './tasks';

export interface ReflectSquadMember {
  actor: ReflectActor;
  role: string;
  model?: string | null;
  systemPrompt: string;
  allowedTools: string[];
}

export interface ReflectSquadSpec {
  name: string;
  description?: string | null;
  leaderActor: ReflectActor;
  members: ReflectSquadMember[];
  createdAtMs: number;
}

export function reflect_list_squads(): Promise<ReflectSquadSpec[]> {
  return invoke('reflect_list_squads');
}

export function reflect_create_squad(spec: ReflectSquadSpec): Promise<void> {
  return invoke('reflect_create_squad', { spec });
}

export function reflect_get_squad(name: string): Promise<ReflectSquadSpec> {
  return invoke('reflect_get_squad', { name });
}

export function reflect_delete_squad(name: string): Promise<void> {
  return invoke('reflect_delete_squad', { name });
}

export function reflect_delegate_next(
  name: string,
  leaderActorId: string,
): Promise<ReflectTask | null> {
  return invoke('reflect_delegate_next', { name, leaderActorId });
}

export function reflect_assign_squad_task(
  name: string,
  taskId: number,
  assignee: string | null,
): Promise<ReflectTask> {
  return invoke('reflect_assign_squad_task', { name, taskId, assignee });
}
