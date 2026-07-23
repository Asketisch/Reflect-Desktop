/**
 * Vitest 测试环境 —— 模拟 Tauri 2 API。
 *
 * useSessions hook 依赖 `invoke('reflect_list_sessions')` 和
 * `invoke('reflect_rename_session', ...)`;
 * useAgent hook 依赖 `invoke('reflect_submit', ...)` + `listen('reflect_event', ...)`。
 *
 * 我们用 fake timers + 手动 resolve 来模拟后端响应。
 */

import { afterEach, beforeEach, vi } from 'vitest';
import { cleanup, render as rtlRender, type RenderOptions } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';

// ====== Tauri invoke mock ======

type InvokeHandler = (...args: any[]) => Promise<unknown>;

const invokeHandlers = new Map<string, InvokeHandler>();

export function mockInvoke(cmd: string, handler: InvokeHandler) {
  invokeHandlers.set(cmd, handler);
}

export function resetMockInvoke() {
  invokeHandlers.clear();
}

// Default handlers
beforeEach(() => {
  resetMockInvoke();

  // reflect_list_sessions — 返回 3 个 stub session
  mockInvoke('reflect_list_sessions', async () => [
    {
      session_id: 'sess-1',
      thread_id: 'thread-1',
      model: 'stub/test',
      provider: 'local',
      started_at: new Date(Date.now() - 3600000).toISOString(),
      message_count: 2,
      tool_count: 0,
      token_total: 0,
      cwd: '/tmp',
      display_name: 'First session',
    },
    {
      session_id: 'sess-2',
      thread_id: 'thread-2',
      model: 'stub/test',
      provider: 'local',
      started_at: new Date(Date.now() - 86400000).toISOString(),
      message_count: 5,
      tool_count: 1,
      token_total: 120,
      cwd: '/tmp',
      display_name: 'Second session',
    },
    {
      session_id: 'sess-3',
      thread_id: 'thread-3',
      model: 'stub/test',
      provider: 'local',
      started_at: new Date(Date.now() - 172800000).toISOString(),
      message_count: 1,
      tool_count: 0,
      token_total: 0,
      cwd: '/tmp',
      display_name: null,
    },
  ]);

  // reflect_rename_session
  mockInvoke('reflect_rename_session', async (_id: string, _new_name: string) => {});

  // reflect_set_permission_mode
  mockInvoke('reflect_set_permission_mode', async () => {});

  // reflect_submit
  mockInvoke('reflect_submit', async (submission: { id: string }) => submission.id);

  // ping
  mockInvoke('ping', async () => ({ msg: 'pong', version: '0.1.0' }));
});

// ====== Mock @tauri-apps/api/core ======

vi.mock('@tauri-apps/api/core', () => {
  const actual = vi.importActual('@tauri-apps/api/core');
  return {
    ...actual,
    invoke: async function (cmd: string, args?: unknown) {
      const handler = invokeHandlers.get(cmd);
      if (handler) return handler(cmd, args);
      throw new Error(`[mock] invoke('${cmd}') not mocked`);
    },
  };
});

// ====== Mock @tauri-apps/api/event ======

type EventHandler = (payload: any) => void;
const eventHandlers = new Map<string, Set<EventHandler>>();

export function emitMockEvent(event: string, payload: unknown) {
  const handlers = eventHandlers.get(event);
  if (handlers) {
    for (const fn of handlers) fn({ payload });
  }
}

export function resetMockEvents() {
  eventHandlers.clear();
}

vi.mock('@tauri-apps/api/event', () => {
  const actual = vi.importActual('@tauri-apps/api/event');
  return {
    ...actual,
    listen: async function (event: string, handler: (e: { payload: unknown }) => void) {
      if (!eventHandlers.has(event)) {
        eventHandlers.set(event, new Set());
      }
      eventHandlers.get(event)!.add(handler);
      return () => {
        eventHandlers.get(event)?.delete(handler);
      };
    },
  };
});

afterEach(() => {
  cleanup();
  resetMockEvents();
  vi.clearAllMocks();
});

// ====== Test utilities ======

/**
 * 创建用于测试的 QueryClient。
 * 每个测试独立实例,避免缓存泄漏。
 */
export function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
}

/**
 * 包裹组件进 QueryClientProvider,用于需要 TanStack Query 的测试。
 */
export function renderWithQueryClient(
  ui: ReactNode,
  options?: Omit<RenderOptions, 'wrapper'>,
) {
  const queryClient = createTestQueryClient();
  return rtlRender(ui, {
    wrapper: ({ children }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
    ...options,
  });
}
