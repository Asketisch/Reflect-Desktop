/**
 * Vitest — useAgent hook + handle_event 纯函数测试。
 *
 * 验证:
 * 1. handle_event: session_configured 设置 session
 * 2. handle_event: agent_message_delta 累积 reply
 * 3. handle_event: agent_message 替换 reply + done
 * 4. handle_event: turn_complete 标记 done
 * 5. handle_event: unknown event 不改变 state
 * 6. useAgent hook: 初始状态
 * 7. useAgent hook: submit 创建 turn + 调用 reflect_submit
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAgent, handle_event, type Turn } from '@/services/agent';
import { mockInvoke, resetMockInvoke } from '@/test/setup.tsx';

// ====== handle_event 纯函数测试 ======

describe('handle_event', () => {
  it('session_configured sets session', () => {
    let session: { model: string; provider: string } | null = null;
    const setSession = (v: unknown) => { session = v as typeof session; };

    handle_event(
      () => {},
      setSession,
      {
        id: '',
        msg: { type: 'session_configured', model: 'stub/test', provider: 'local' },
      },
    );

    expect(session).toEqual({ model: 'stub/test', provider: 'local' });
  });

  it('agent_message_delta accumulates reply', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: '', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'agent_message_delta', delta: 'Hello' } },
    );
    expect(state.turns[0].reply).toBe('Hello');

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'agent_message_delta', delta: ' world' } },
    );
    expect(state.turns[0].reply).toBe('Hello world');
  });

  it('agent_message replaces reply and marks done', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: 'partial', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'agent_message', text: 'Full response' } },
    );

    expect(state.turns[0].reply).toBe('Full response');
    expect(state.turns[0].done).toBe(true);
  });

  it('turn_complete marks done', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: '', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'turn_complete', turn_id: 'turn-1', usage: {}, status: 'success' } },
    );

    expect(state.turns[0].done).toBe(true);
  });

  it('ignores unknown event types', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: 'hello', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'turn_started', turn_id: 'turn-1' } },
    );

    expect(state.turns[0].reply).toBe('hello');
    expect(state.turns[0].done).toBe(false);
  });

  it('session_configured ignores empty model/provider', () => {
    let session: { model: string; provider: string } | null = { model: 'old', provider: 'old' };
    const setSession = (v: unknown) => { session = v as typeof session; };

    handle_event(
      () => {},
      setSession,
      { id: '', msg: { type: 'session_configured', model: '', provider: '' } },
    );

    // No update because both fields empty
    expect(session).toEqual({ model: 'old', provider: 'old' });
  });

  it('agent_message_delta with missing id does not throw', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: '', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    // id matches nothing → no-op for the array (still safe)
    expect(() => {
      handle_event(
        setTurns,
        () => {},
        { id: 'no-such-id', msg: { type: 'agent_message_delta', delta: 'x' } },
      );
    }).not.toThrow();
  });

  it('agent_message_delta tolerates missing delta field', () => {
    const state = { turns: [{ id: 't1', user: 'hi', reply: 'base', done: false }] };
    const setTurns = (v: unknown) => {
      state.turns = typeof v === 'function'
        ? (v as (prev: Turn[]) => Turn[])(state.turns)
        : (v as Turn[]);
    };

    handle_event(
      setTurns,
      () => {},
      { id: 't1', msg: { type: 'agent_message_delta' } as unknown as { type: 'agent_message_delta'; delta: string } },
    );
    expect(state.turns[0].reply).toBe('base');
  });
});

// ====== useAgent hook 测试 ======

describe('useAgent', () => {
  beforeEach(() => {
    resetMockInvoke();
    mockInvoke('reflect_submit', async (submission: { id: string }) => submission.id);
  });

  it('starts with empty turns and null session', () => {
    const { result } = renderHook(() => useAgent());

    expect(result.current.turns).toEqual([]);
    expect(result.current.session).toBeNull();
  });

  it('submit creates a turn and calls reflect_submit', async () => {
    const { result } = renderHook(() => useAgent());

    await act(async () => {
      await result.current.submit('hello world');
    });

    expect(result.current.turns.length).toBe(1);
    expect(result.current.turns[0].user).toBe('hello world');
    expect(result.current.turns[0].reply).toBe('');
    expect(result.current.turns[0].done).toBe(false);
  });
});
