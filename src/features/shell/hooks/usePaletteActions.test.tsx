/**
 * usePaletteActions —— 命令面板动作编排测试。
 *
 * 覆盖:
 *   - newSession 触发 onNewChat + pushToast(info)
 *   - exportActive 无 activeId 时推 warn toast,有 activeId 时调 reflect_export_session
 *   - saveConfig 调 reflect_save_config('') 并推 success toast
 *   - runSlash 调 submit
 *   - clearAllSessions 复用 qc 缓存的 sessions
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { usePaletteActions, type PaletteToasts } from './usePaletteActions';
import { useAgentStore } from '@/stores/agentStore';
import { mockInvoke, resetMockInvoke } from '@/test/setup';

const TOASTS: PaletteToasts = {
  newSession: 'New session started.',
  noSessionsToClear: 'No sessions to clear.',
  deleteFailed: (msg) => `Delete failed: ${msg}`,
  clearedSessions: (count) => `Cleared ${count} session(s).`,
  noActiveToExport: 'No active session to export.',
  exported: (path) => `Exported → ${path}`,
  exportedNoPath: 'Exported → (no path)',
  exportFailed: (msg) => `Export failed: ${msg}`,
  configReloaded: 'Config reloaded.',
  saveFailed: (msg) => `Save failed: ${msg}`,
};

function wrapper(qc: QueryClient) {
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe('usePaletteActions', () => {
  let qc: QueryClient;
  let onNewChat: () => void;

  beforeEach(() => {
    resetMockInvoke();
    useAgentStore.getState().reset();
    qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    onNewChat = vi.fn();
  });
  afterEach(() => {
    resetMockInvoke();
  });

  it('newSession: invokes onNewChat + info toast', () => {
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    act(() => result.current.newSession());
    expect(onNewChat).toHaveBeenCalledTimes(1);
    // toast pushed to store
    const toasts = useAgentStore.getState().toasts;
    expect(toasts.length).toBeGreaterThan(0);
    expect(toasts[toasts.length - 1].kind).toBe('info');
    expect(toasts[toasts.length - 1].message).toBe('New session started.');
  });

  it('exportActive: no activeId → warn toast', async () => {
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.exportActive();
    });
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].kind).toBe('warn');
    expect(toasts[toasts.length - 1].message).toBe('No active session to export.');
  });

  it('exportActive: success path calls reflect_export_session + success toast', async () => {
    const spy = vi.fn(async () => '/tmp/foo.json');
    mockInvoke('reflect_export_session', spy);

    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: 'sess-1',
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.exportActive();
    });
    // Bridge forwards both cmd + args to handler; verify id was passed.
    expect(spy).toHaveBeenCalledWith(
      'reflect_export_session',
      expect.objectContaining({ id: 'sess-1' }),
    );

    const toasts = useAgentStore.getState().toasts;
    const last = toasts[toasts.length - 1];
    expect(last.kind).toBe('success');
    expect(last.message).toBe('Exported → /tmp/foo.json');
  });

  it('exportActive: null path uses exportedNoPath', async () => {
    mockInvoke('reflect_export_session', async () => null);

    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: 'sess-1',
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.exportActive();
    });
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].message).toBe('Exported → (no path)');
  });

  it('saveConfig: success toast', async () => {
    mockInvoke('reflect_save_config', async () => undefined);
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.saveConfig();
    });
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].message).toBe('Config reloaded.');
  });

  it('saveConfig: failure toast', async () => {
    mockInvoke('reflect_save_config', async () => {
      throw new Error('boom');
    });
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.saveConfig();
    });
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].kind).toBe('error');
    expect(toasts[toasts.length - 1].message).toBe('Save failed: boom');
  });

  it('runSlash: invokes submit fire-and-forget', () => {
    const submitSpy = vi.spyOn(useAgentStore.getState(), 'submit');
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    act(() => result.current.runSlash('/compact'));
    expect(submitSpy).toHaveBeenCalledWith('/compact');
    submitSpy.mockRestore();
  });

  it('onNewChat hook forwards newSession call', () => {
    const localNewChat = vi.fn();
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat: localNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    act(() => result.current.newSession());
    expect(localNewChat).toHaveBeenCalledTimes(1);
  });

  it('clearAllSessions: fetches sessions and deletes each', async () => {
    // Each test starts with resetMockInvoke() — register our own reflect_list_sessions.
    mockInvoke('reflect_list_sessions', async () => [
      { session_id: 'sess-1' },
      { session_id: 'sess-2' },
      { session_id: 'sess-3' },
    ]);
    const delSpy = vi.fn(async () => undefined);
    mockInvoke('reflect_delete_session', delSpy);

    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.clearAllSessions();
    });
    expect(delSpy).toHaveBeenCalledTimes(3);
    expect(delSpy).toHaveBeenCalledWith(
      'reflect_delete_session',
      expect.objectContaining({ id: 'sess-1' }),
    );
    expect(delSpy).toHaveBeenCalledWith(
      'reflect_delete_session',
      expect.objectContaining({ id: 'sess-2' }),
    );

    // success toast contains count
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].message).toBe('Cleared 3 session(s).');
    // query invalidated by hook
    await waitFor(() => {
      const state = qc.getQueryState(['sessions']);
      expect(state?.isInvalidated).toBe(true);
    });
  });

  it('clearAllSessions: empty list → info toast', async () => {
    mockInvoke('reflect_list_sessions', async () => []);
    const { result } = renderHook(
      () =>
        usePaletteActions({
          activeId: null,
          onNewChat,
          toasts: TOASTS,
        }),
      { wrapper: wrapper(qc) },
    );
    await act(async () => {
      await result.current.clearAllSessions();
    });
    const toasts = useAgentStore.getState().toasts;
    expect(toasts[toasts.length - 1].message).toBe('No sessions to clear.');
  });
});