/**
 * Vitest — pushToast / dismissToast actions。
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useAgentStore } from '@/stores/agentStore';

describe('agentStore pushToast / dismissToast', () => {
  beforeEach(() => {
    useAgentStore.setState({ toasts: [] });
  });

  it('pushToast adds a toast with unique id', () => {
    const id1 = useAgentStore.getState().pushToast({ kind: 'info', message: 'a' });
    const id2 = useAgentStore.getState().pushToast({ kind: 'warn', message: 'b' });
    expect(id1).not.toBe(id2);
    expect(useAgentStore.getState().toasts).toHaveLength(2);
    expect(useAgentStore.getState().toasts[0].message).toBe('a');
    expect(useAgentStore.getState().toasts[1].kind).toBe('warn');
  });

  it('dismissToast removes by id', () => {
    const id = useAgentStore.getState().pushToast({ kind: 'error', message: 'x' });
    useAgentStore.getState().dismissToast(id);
    expect(useAgentStore.getState().toasts).toHaveLength(0);
  });

  it('auto-dismisses after ttlMs (uses fake timers)', () => {
    vi.useFakeTimers();
    useAgentStore.getState().pushToast({ kind: 'info', message: 'temp', ttlMs: 1000 });
    expect(useAgentStore.getState().toasts).toHaveLength(1);
    vi.advanceTimersByTime(1100);
    expect(useAgentStore.getState().toasts).toHaveLength(0);
    vi.useRealTimers();
  });

  it('ttlMs=0 keeps the toast sticky', () => {
    vi.useFakeTimers();
    useAgentStore.getState().pushToast({ kind: 'info', message: 'sticky', ttlMs: 0 });
    vi.advanceTimersByTime(60_000);
    expect(useAgentStore.getState().toasts).toHaveLength(1);
    vi.useRealTimers();
  });
});
