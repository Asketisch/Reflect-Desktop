/**
 * Vitest — useSessions hook 测试。
 *
 * 验证:
 * 1. 挂载后自动调用 reflect_list_sessions
 * 2. 时间分桶正确 (Now / Today / Yesterday / This week / Older)
 * 3. refresh 重新拉取
 * 4. rename 调用 reflect_rename_session + refresh
 * 5. 错误处理 (invoke reject)
 * 6. activeId 选择
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { useSessions } from '@/features/sessions/hooks/useSessions';
import { mockInvoke, resetMockInvoke, createTestQueryClient } from '@/test/setup.tsx';

function hookWrapper({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={createTestQueryClient()}>
      {children}
    </QueryClientProvider>
  );
}

describe('useSessions', () => {
  beforeEach(() => {
    resetMockInvoke();
    // Default: return 3 sessions at different times
    const now = Date.now();
    mockInvoke('reflect_list_sessions', async () => [
      {
        session_id: 's1',
        thread_id: 't1',
        model: 'stub/test',
        provider: 'local',
        started_at: new Date(now - 30 * 60 * 1000).toISOString(),
        message_count: 2,
        tool_count: 0,
        token_total: 0,
        cwd: '/tmp',
        display_name: 'Recent',
      },
      {
        session_id: 's2',
        thread_id: 't2',
        model: 'stub/test',
        provider: 'local',
        started_at: new Date(now - 3 * 60 * 60 * 1000).toISOString(),
        message_count: 5,
        tool_count: 1,
        token_total: 120,
        cwd: '/tmp',
        display_name: 'Today',
      },
      {
        session_id: 's3',
        thread_id: 't3',
        model: 'stub/test',
        provider: 'local',
        started_at: new Date(now - 10 * 24 * 60 * 60 * 1000).toISOString(),
        message_count: 1,
        tool_count: 0,
        token_total: 0,
        cwd: '/tmp',
        display_name: null,
      },
    ]);
  });

  it('fetches sessions on mount', async () => {
    let invoked = false;
    mockInvoke('reflect_list_sessions', async () => {
      invoked = true;
      return [];
    });

    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(invoked).toBe(true), { timeout: 2000 });
    expect(result.current.error).toBeNull();
    expect(result.current.buckets).toEqual([]);
  });

  it('buckets sessions by time', async () => {
    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(result.current.buckets.length).toBeGreaterThan(0), { timeout: 2000 });

    const labels = result.current.buckets.map((b) => b.label);
    expect(labels).toContain('Now');
    expect(labels).toContain('Today');
    expect(labels).toContain('Older');
  });

  it('calls reflect_rename_session on rename', async () => {
    let renameInvoked = false;
    mockInvoke('reflect_rename_session', async () => {
      renameInvoked = true;
    });

    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(result.current.buckets.length).toBeGreaterThan(0), { timeout: 2000 });

    await act(async () => {
      await result.current.rename('s1', 'new name');
    });

    expect(renameInvoked).toBe(true);
  });

  it('sets error on invoke failure', async () => {
    mockInvoke('reflect_list_sessions', async () => {
      throw new Error('network error');
    });

    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(!!result.current.error).toBe(true), { timeout: 2000 });
    // TanStack Query error may be Error or string depending on version
    const err = result.current.error;
    const msg = typeof err === 'string' ? err : ((err as Error | null)?.message ?? String(err as unknown as Record<string, string>));
    expect(msg).toContain('network error');
  });

  it('supports activeId selection', async () => {
    mockInvoke('reflect_list_sessions', async () => []);

    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(result.current.buckets.length).toBe(0), { timeout: 2000 });

    act(() => result.current.setActiveId('s1'));
    expect(result.current.activeId).toBe('s1');
  });

  it('refresh re-fetches sessions', async () => {
    let callCount = 0;
    mockInvoke('reflect_list_sessions', async () => {
      callCount += 1;
      return [];
    });

    const { result } = renderHook(() => useSessions(), {
      wrapper: hookWrapper,
    });

    await waitFor(() => expect(callCount).toBe(1), { timeout: 2000 });

    act(() => result.current.refetch());

    await waitFor(() => expect(callCount).toBe(2), { timeout: 2000 });
  });
});
