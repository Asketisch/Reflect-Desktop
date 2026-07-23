/**
 * 14 个 reflect_* command —— 每个 Op 一个 wrapper。
 *
 * CodexMonitor 同名: `src/services/tauri.ts::commands` 节
 *
 * 设计蓝图：`docs/PROTOCOL_BRIDGE.md` §2 + §6。
 */
import { invoke, listen } from './bridge';
import type { ReflectRolloutRecord, ReflectSessionInfo, ReviewDecision } from './types';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';

/** Health. */
export async function ping(): Promise<{ msg: string; version: string }> {
  return invoke<{ msg: string; version: string }>('ping');
}

/** Submission 入口。返回 submission.id。 */
export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return invoke<string>('reflect_submit', { submission: s });
}

/** Lifecycle / turn 控制。Op 命令返回 submission id(供前端 pairing/调试)。 */
export async function reflect_interrupt(): Promise<void> {
  return invoke<void>('reflect_interrupt');
}
export async function reflect_compact(): Promise<string> {
  return invoke<string>('reflect_compact');
}
export async function reflect_rewind(to_turn_id?: string): Promise<string> {
  return invoke<string>('reflect_rewind', { toTurnId: to_turn_id ?? null });
}
export async function reflect_shutdown(): Promise<string> {
  return invoke<string>('reflect_shutdown');
}

/** Approvals. */
export async function reflect_tool_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_tool_approval', { id, decision });
}
export async function reflect_hook_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_hook_approval', { id, decision });
}
export async function reflect_plan_approval(id: string, decision: ReviewDecision): Promise<string> {
  return invoke<string>('reflect_plan_approval', { id, decision });
}

/** Plan mode. */
export async function reflect_enter_plan_mode(task: string): Promise<string> {
  return invoke<string>('reflect_enter_plan_mode', { task });
}
export async function reflect_exit_plan_mode(): Promise<string> {
  return invoke<string>('reflect_exit_plan_mode');
}

/** Effort / permission. */
export async function reflect_set_effort(level: string): Promise<string> {
  return invoke<string>('reflect_set_effort', { level });
}
export async function reflect_set_permission_mode(mode: string): Promise<string> {
  return invoke<string>('reflect_set_permission_mode', { mode });
}
export async function reflect_cycle_permission_mode(): Promise<string> {
  return invoke<string>('reflect_cycle_permission_mode');
}

/** Ask user. */
export async function reflect_ask_user_question_response(id: string, answers: unknown): Promise<string> {
  return invoke<string>('reflect_ask_user_question_response', { id, answers });
}
export async function reflect_ask_user_input_response(id: string, text: string): Promise<string> {
  return invoke<string>('reflect_ask_user_input_response', { id, text });
}

// ====== 诊断 / config / tools(阶段 2 新增) ======

/** agent 状态快照 —— 前端状态徽标 + 降级引导用。 */
export interface ReflectAgentStatus {
  ready: boolean;
  has_model: boolean;
  model: string;
  workspace: string;
  degraded_reason: string | null;
}

/** 单个工具的 name + description。 */
export interface ReflectToolInfo {
  name: string;
  description: string;
}

/** 返回 agent 状态(ready / has_model / model / workspace / degraded_reason)。 */
export async function reflect_agent_status(): Promise<ReflectAgentStatus> {
  return invoke<ReflectAgentStatus>('reflect_agent_status');
}

/** 读取 ~/.reflect/config.toml 的 TOML 字符串。Settings 页加载用。 */
export async function reflect_get_config(): Promise<string> {
  return invoke<string>('reflect_get_config');
}

/** 写回 ~/.reflect/config.toml(写盘前校验合法性)。 */
export async function reflect_save_config(toml: string): Promise<void> {
  return invoke<void>('reflect_save_config', { toml });
}

/** 列出当前 ToolRegistry 中所有工具(name + description)。 */
export async function reflect_list_tools(): Promise<ReflectToolInfo[]> {
  return invoke<ReflectToolInfo[]>('reflect_list_tools');
}

/** Session I/O. */
export async function reflect_list_sessions(
  opts: { limit?: number; offset?: number } = {},
): Promise<ReflectSessionInfo[]> {
  return invoke<ReflectSessionInfo[]>('reflect_list_sessions', {
    limit: opts.limit ?? null,
    offset: opts.offset ?? null,
  });
}
export async function reflect_rename_session(id: string, new_name: string): Promise<void> {
  return invoke<void>('reflect_rename_session', { id, newName: new_name });
}
export async function reflect_delete_session(id: string): Promise<void> {
  return invoke<void>('reflect_delete_session', { id });
}
export async function reflect_replay_session(id: string): Promise<ReflectRolloutRecord[]> {
  return invoke<ReflectRolloutRecord[]>('reflect_replay_session', { id });
}
/** Export session to JSON, return absolute path. */
export async function reflect_export_session(id: string): Promise<string | null> {
  return invoke<string | null>('reflect_export_session', { id });
}

/** 订阅 reflect_event 通道（payload = ReflectEvent）。 */
export async function onReflectEvent(handler: (event: ReflectEvent) => void) {
  return listen<ReflectEvent>('reflect_event', (e) => handler(e.payload));
}

// ====== B1-07 / B9-06: Workspace management ======

export interface ReflectWorkspaceInfo {
  path: string;
  label: string;
  last_used: number;
  session_count: number;
}

export async function reflect_list_workspaces(): Promise<ReflectWorkspaceInfo[]> {
  return invoke<ReflectWorkspaceInfo[]>('reflect_list_workspaces');
}
export async function reflect_set_workspace(path: string): Promise<void> {
  return invoke<void>('reflect_set_workspace', { path });
}
export async function reflect_current_workspace(): Promise<string> {
  return invoke<string>('reflect_current_workspace');
}

// ====== B1-07 / B11-06: Skills ======

export interface ReflectSkillInfo {
  name: string;
  description: string;
  path: string;
  tools: string[];
  triggers: string[];
}

export async function reflect_list_skills(): Promise<ReflectSkillInfo[]> {
  return invoke<ReflectSkillInfo[]>('reflect_list_skills');
}

// ====== B1-07 / B11-01: Memory ======

export interface ReflectMemoryEntry {
  scope: string;
  key: string;
  value: string;
}

export async function reflect_list_memory(): Promise<ReflectMemoryEntry[]> {
  return invoke<ReflectMemoryEntry[]>('reflect_list_memory');
}
export async function reflect_add_memory(scope: string, key: string, value: string): Promise<void> {
  return invoke<void>('reflect_add_memory', { scope, key, value });
}
export async function reflect_remove_memory(scope: string, key: string): Promise<void> {
  return invoke<void>('reflect_remove_memory', { scope, key });
}

// ====== B1-07 / B11-02: Hooks ======

export interface ReflectHookInfo {
  name: string;
  kind: string;
  enabled: boolean;
  config_summary: string;
}

export async function reflect_list_hooks(): Promise<ReflectHookInfo[]> {
  return invoke<ReflectHookInfo[]>('reflect_list_hooks');
}
export async function reflect_toggle_hook(name: string, enabled: boolean): Promise<void> {
  return invoke<void>('reflect_toggle_hook', { name, enabled });
}