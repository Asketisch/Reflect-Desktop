/**
 * Vitest — agentStore reducer (reduceEvent) + useAgent 兼容 hook。
 *
 * 阶段 3a:Turn 模型从 `{user,reply,done}` 升级为 `{id,items,status}`,
 * 本测试覆盖新 reducer 的核心路径。
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

import { useAgentStore, reduceEvent, type AgentState, type Turn } from '@/stores/agentStore';
import { useAgent } from '@/services/agent';
import type { ReflectEvent } from '@/types/protocol';
import { mockInvoke, resetMockInvoke } from '@/test/setup.tsx';

/** 构造一个最小可用 state(空 turns + null session)。 */
function emptyState(): AgentState {
  return useAgentStore.getState();
}

/** 构造一个带单个空 turn 的 state,便于 delta/finalize 测试。 */
function stateWithTurn(id = 't1'): AgentState {
  const base = emptyState();
  return {
    ...base,
    turns: [{ id, items: [{ kind: 'user_text', text: 'hi' }], status: 'streaming' }],
  };
}

const ev = (id: string, msg: Record<string, unknown>): ReflectEvent =>
  ({ id, msg: { type: msg.type, ...msg } } as ReflectEvent);

// ====== reduceEvent 纯函数测试 ======

describe('reduceEvent', () => {
  it('session_configured sets session', () => {
    const patch = reduceEvent(emptyState(), ev('', { type: 'session_configured', model: 'anthropic/claude', provider: 'anthropic' }));
    expect(patch.session).toEqual({ model: 'anthropic/claude', provider: 'anthropic' });
  });

  it('session_configured ignores empty model', () => {
    const patch = reduceEvent(emptyState(), ev('', { type: 'session_configured', model: '', provider: '' }));
    expect(patch.session).toBeUndefined();
  });

  it('agent_message_delta accumulates into assistant_text', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'agent_message_delta', delta: 'Hello' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'agent_message_delta', delta: ' world' })) };
    const textItem = s.turns[0].items.find((i) => i.kind === 'assistant_text');
    expect(textItem).toBeDefined();
    expect(textItem && textItem.kind === 'assistant_text' && textItem.text).toBe('Hello world');
  });

  it('agent_message finalizes assistant_text + keeps streaming=false', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'agent_message_delta', delta: 'partial' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'agent_message', text: 'Full response' })) };
    const textItem = s.turns[0].items.find((i) => i.kind === 'assistant_text');
    expect(textItem && textItem.kind === 'assistant_text' && textItem.text).toBe('Full response');
    expect(textItem && textItem.kind === 'assistant_text' && textItem.streaming).toBe(false);
  });

  it('turn_complete marks status=done', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'turn_complete' })) };
    expect(s.turns[0].status).toBe('done');
  });

  it('turn_aborted marks status=aborted', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'turn_aborted' })) };
    expect(s.turns[0].status).toBe('aborted');
  });

  it('tool_call_begin + tool_call_end produce tool_call + tool_output items', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'tool_call_begin', call_id: 'c1', tool_name: 'bash', args: { command: 'ls' } })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'tool_call_end', call_id: 'c1', output: 'file1\nfile2' })) };
    const callItem = s.turns[0].items.find((i) => i.kind === 'tool_call');
    const outItem = s.turns[0].items.find((i) => i.kind === 'tool_output');
    expect(callItem && callItem.kind === 'tool_call' && callItem.toolName).toBe('bash');
    expect(callItem && callItem.kind === 'tool_call' && callItem.status).toBe('done');
    expect(outItem && outItem.kind === 'tool_output' && outItem.text).toBe('file1\nfile2');
  });

  it('thinking_delta accumulates into thinking item', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'thinking_delta', delta: 'Hmm' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'thinking_delta', delta: '...' })) };
    const thinkItem = s.turns[0].items.find((i) => i.kind === 'thinking');
    expect(thinkItem && thinkItem.kind === 'thinking' && thinkItem.text).toBe('Hmm...');
  });

  it('error on turn appends error item; session-level error sets lastError', () => {
    let s = stateWithTurn('t1');
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'error', message: 'boom' })) };
    expect(s.turns[0].items.some((i) => i.kind === 'error' && i.text === 'boom')).toBe(true);

    // session-level (id='') 不挂 turn,设 lastError。
    const base = emptyState();
    const patch = reduceEvent(base, ev('', { type: 'error', message: 'session boom' }));
    expect(patch.lastError).toBe('session boom');
  });

  it('approval_request enqueues pending approval', () => {
    const s = emptyState();
    const patch = reduceEvent(s, ev('t1', { type: 'approval_request', id: 'a1', kind: 'tool', tool_name: 'bash' }));
    expect(patch.pendingApprovals).toBeDefined();
    expect(patch.pendingApprovals![0]).toMatchObject({ id: 'a1', kind: 'tool', toolName: 'bash' });
  });

  it('mcp_server_started upserts mcpServers', () => {
    let s = emptyState();
    s = { ...s, ...reduceEvent(s, ev('', { type: 'mcp_server_started', name: 'fs' })) };
    s = { ...s, ...reduceEvent(s, ev('', { type: 'mcp_server_failed', name: 'fs', error: 'died' })) };
    expect(s.mcpServers.find((m) => m.name === 'fs')?.status).toBe('failed');
  });

  it('permission_mode_changed updates permissionMode', () => {
    const s = emptyState();
    const patch = reduceEvent(s, ev('', { type: 'permission_mode_changed', mode: 'plan' }));
    expect(patch.permissionMode).toBe('plan');
  });

  it('unknown event returns empty patch (no state change)', () => {
    const s = stateWithTurn('t1');
    const patch = reduceEvent(s, ev('t1', { type: 'token_count' }));
    expect(Object.keys(patch).length).toBe(0);
  });
});

// ====== useAgent 兼容 hook 测试 ======

describe('useAgent (compat hook from store)', () => {
  beforeEach(() => {
    useAgentStore.getState().reset();
    resetMockInvoke();
    mockInvoke('reflect_submit', async (submission: { id: string }) => submission.id);
  });

  it('starts with empty turns and null session', () => {
    const { result } = renderHook(() => useAgent());
    expect(result.current.turns).toEqual([]);
    expect(result.current.session).toBeNull();
  });

  it('submit appends a turn with user_text item', async () => {
    const { result } = renderHook(() => useAgent());
    await act(async () => {
      await result.current.submit('hello world');
    });
    expect(result.current.turns.length).toBe(1);
    const turn: Turn = result.current.turns[0];
    expect(turn.items[0]).toMatchObject({ kind: 'user_text', text: 'hello world' });
    expect(turn.status).toBe('streaming');
  });
});
