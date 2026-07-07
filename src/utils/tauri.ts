/**
 * Tauri 2 IPC bridge —— 对应 CodexMonitor 的 `src/services/tauri.ts`。
 *
 * - `invoke` 包装: 自动处理脱离 Tauri 的情况(普通浏览器下 invoke 不存在),
 *   返回 fallback 而非 throw(参考 CodexMonitor `isMissingTauriInvokeError` guard)。
 * - `listen` 包装: 同上。
 * - 14 个 reflect_* command 一个函数一个,带类型签名。
 *
 * 设计蓝图:`docs/gui/03-architecture.md` §5.3。
 */
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';

export interface ReflectSessionInfo {
  session_id: string;
  thread_id: string;
  model: string;
  provider: string;
  started_at: string;
  message_count: number;
  tool_count: number;
  token_total: number;
  cwd: string;
  display_name?: string;
}

export interface ReflectRolloutRecord {
  type: string;
  [k: string]: unknown;
}

/** 14 个 Op commands + ping 占位。 */
export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return invoke<string>('reflect_submit', { submission: s });
}

export async function reflect_interrupt(): Promise<void> {
  return invoke<void>('reflect_interrupt');
}

export async function reflect_compact(): Promise<void> {
  return invoke<void>('reflect_compact');
}

export async function reflect_rewind(to_turn_id?: string): Promise<void> {
  return invoke<void>('reflect_rewind', { toTurnId: to_turn_id ?? null });
}

export async function reflect_shutdown(): Promise<void> {
  return invoke<void>('reflect_shutdown');
}

export async function reflect_tool_approval(
  id: string,
  decision: 'approve' | 'deny' | 'abort',
): Promise<void> {
  return invoke<void>('reflect_tool_approval', { id, decision });
}

export async function reflect_hook_approval(
  id: string,
  decision: 'approve' | 'deny' | 'abort',
): Promise<void> {
  return invoke<void>('reflect_hook_approval', { id, decision });
}

export async function reflect_enter_plan_mode(task: string): Promise<void> {
  return invoke<void>('reflect_enter_plan_mode', { task });
}

export async function reflect_exit_plan_mode(): Promise<void> {
  return invoke<void>('reflect_exit_plan_mode');
}

export async function reflect_plan_approval(
  id: string,
  decision: 'approve' | 'deny',
): Promise<void> {
  return invoke<void>('reflect_plan_approval', { id, decision });
}

export async function reflect_set_effort(level: string): Promise<void> {
  return invoke<void>('reflect_set_effort', { level });
}

export async function reflect_ask_user_question_response(
  id: string,
  answers: unknown,
): Promise<void> {
  return invoke<void>('reflect_ask_user_question_response', { id, answers });
}

export async function reflect_ask_user_input_response(
  id: string,
  text: string,
): Promise<void> {
  return invoke<void>('reflect_ask_user_input_response', { id, text });
}

export async function reflect_set_permission_mode(mode: string): Promise<void> {
  return invoke<void>('reflect_set_permission_mode', { mode });
}

export async function reflect_cycle_permission_mode(): Promise<void> {
  return invoke<void>('reflect_cycle_permission_mode');
}

// ----- M1.3 session 列表 -----

export async function reflect_list_sessions(): Promise<ReflectSessionInfo[]> {
  return invoke<ReflectSessionInfo[]>('reflect_list_sessions');
}

export async function reflect_rename_session(
  id: string,
  new_name: string,
): Promise<void> {
  return invoke<void>('reflect_rename_session', { id, newName: new_name });
}

export async function reflect_replay_session(
  id: string,
): Promise<ReflectRolloutRecord[]> {
  return invoke<ReflectRolloutRecord[]>('reflect_replay_session', { id });
}

/** Subscribe to "reflect_event" channel. Returns unlisten fn. */
export async function onReflectEvent(
  handler: (event: ReflectEvent) => void,
): Promise<UnlistenFn> {
  return listen<ReflectEvent>('reflect_event', (e) => handler(e.payload));
}

/** Ping 占位 command — 验证 IPC 通路。 */
export async function ping(): Promise<{ msg: string; version: string }> {
  return invoke<{ msg: string; version: string }>('ping');
}

// ---------------------- internal ----------------------

/**
 * CodexMonitor 风格的 fallback wrapper —— 脱 Tauri(普通浏览器预览)时不 throw,
 * 返回安全 fallback,让 vite dev mode 的 Storybook / 文档示例可用。
 */
function isMissingTauriInvokeError(e: unknown): boolean {
  if (!(e instanceof Error)) return false;
  return /IPC invoke/i.test(e.message) || /not in Tauri context/i.test(e.message);
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (e) {
    if (isMissingTauriInvokeError(e)) {
      // 非 Tauri 上下文(browser preview / Storybook) — 静默 fallback。
      // 实际部署不会命中这里(Tauri 2 IPC 永远存在)。
      console.warn(`[reflect-gui] invoke('${cmd}') outside Tauri context`);
      return undefined as unknown as T;
    }
    throw e;
  }
}

async function listen<T>(
  event: string,
  handler: (e: { payload: T }) => void,
): Promise<UnlistenFn> {
  try {
    return await tauriListen<T>(event, handler);
  } catch (e) {
    if (isMissingTauriInvokeError(e)) {
      console.warn(`[reflect-gui] listen('${event}') outside Tauri context`);
      return () => {};
    }
    throw e;
  }
}
