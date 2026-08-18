/**
 * src/stores/agentStore.test.ts —— 全部 EventMsg 变体的 reducer 覆盖测试（B1-05）。
 *
 * 每个测试为单个变体构造最小化的 `ReflectEvent`，并断言 `reduceEvent`
 * 返回的 patch。本测试文件不演练完整的 store / IPC 层 ——
 * 那由 `src/services/agent.test.ts` 覆盖。目标是**每个变体的
 * reducer 正确性**。
 */
import { describe, it, expect } from 'vitest';
import { reduceEvent, type AgentState } from './agentStore';
import type { ReflectEvent, ReflectEventMsg } from '@/types/protocol';

// ====== Helpers ======

const emptyState = (): AgentState => ({
  turns: [],
  session: null,
  permissionMode: 'auto',
  pendingApprovals: [],
  pendingQuestions: [],
  pendingAskUser: [],
  pendingPlan: null,
  mcpServers: [],
  lspServers: [],
  lastError: null,
  subscribed: false,
  tokens: null,
  contextWindowSize: null,
  collabSessions: [],
  mcpInvocations: [],
  lastRouting: null,
  configReloadedAt: null,
  toasts: [],
  // Actions（reducer 测试中未使用；严格的 AgentState 类型要求它们存在）。
  subscribe: () => () => {},
  submit: async () => {},
  submitItems: async () => {},
  interrupt: async () => {},
  compact: async () => {},
  rewind: async () => {},
  shutdown: async () => {},
  approve: async () => {},
  approvePlan: async () => {},
  enterPlanMode: async () => {},
  exitPlanMode: async () => {},
  setEffort: async () => {},
  setPermissionMode: async () => {},
  cyclePermissionMode: async () => {},
  answerQuestion: async () => {},
  answerInput: async () => {},
  clearError: () => {},
  pushToast: () => '',
  dismissToast: () => {},
  reset: () => {},
});

const ev = (id: string, msg: ReflectEventMsg): ReflectEvent => ({ id, msg });

// ====== Lifecycle (6) ======

describe('reduceEvent — lifecycle', () => {
  it('session_configured sets model+provider', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', {
        type: 'session_configured',
        session_id: 's1',
        model: 'claude-opus-4-7',
        provider: 'anthropic',
        approval_policy: 'auto',
        sandbox_policy: 'workspace_only',
      }),
    );
    expect(patch.session).toEqual({ model: 'claude-opus-4-7', provider: 'anthropic' });
  });

  it('session_configured captures context_window_size', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', {
        type: 'session_configured',
        session_id: 's1',
        model: 'claude-opus-4-7',
        provider: 'anthropic',
        context_window_size: 200000,
        approval_policy: 'auto',
        sandbox_policy: 'workspace_only',
      }),
    );
    expect(patch.contextWindowSize).toBe(200000);
  });

  it('turn_started creates empty turn if missing', () => {
    const patch = reduceEvent(emptyState(), ev('t1', { type: 'turn_started', turn_id: 't1' }));
    expect(patch.turns).toHaveLength(1);
    expect(patch.turns![0]).toMatchObject({ id: 't1', status: 'streaming', items: [] });
  });

  it('turn_started is idempotent', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [{ id: 't1', items: [], status: 'streaming' }],
    };
    const patch = reduceEvent(s, ev('t1', { type: 'turn_started', turn_id: 't1' }));
    expect(patch.turns ?? s.turns).toHaveLength(1);
  });

  it('turn_complete marks turn done', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [{ id: 't1', items: [], status: 'streaming' }],
    };
    const patch = reduceEvent(
      s,
      ev('t1', {
        type: 'turn_complete',
        turn_id: 't1',
        usage: { input_tokens: 0, output_tokens: 0, cached_tokens: 0, cache_write_tokens: 0, total_tokens: 0 },
        status: 'success',
      }),
    );
    expect(patch.turns![0].status).toBe('done');
  });

  it('turn_aborted marks turn aborted', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [{ id: 't1', items: [], status: 'streaming' }],
    };
    const patch = reduceEvent(
      s,
      ev('t1', { type: 'turn_aborted', turn_id: 't1', reason: { type: 'user_interrupt' } }),
    );
    expect(patch.turns![0].status).toBe('aborted');
  });

  it('turn_rewound truncates turns to to_turn_id', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [
        { id: 't1', items: [], status: 'done' },
        { id: 't2', items: [], status: 'done' },
        { id: 't3', items: [], status: 'done' },
      ],
    };
    const patch = reduceEvent(s, ev('', { type: 'turn_rewound', to_turn_id: 't2', truncated_after: 1 }));
    expect(patch.turns).toHaveLength(2);
    expect(patch.turns!.map((t) => t.id)).toEqual(['t1', 't2']);
  });

  it('shutdown_complete sets lastError', () => {
    // shutdown_complete 是单元变体（无 payload 字段）。
    const patch = reduceEvent(emptyState(), ev('', { type: 'shutdown_complete' } as never));
    expect(patch.lastError).toBe('agent shut down');
  });
});

// ====== LLM output (4) ======

describe('reduceEvent — LLM output', () => {
  it('agent_message_delta appends to streaming item', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [{ id: 't1', items: [{ kind: 'assistant_text', text: 'Hello', streaming: true }], status: 'streaming' }],
    };
    const patch = reduceEvent(s, ev('t1', { type: 'agent_message_delta', delta: ' world' }));
    expect(patch.turns![0].items[0]).toMatchObject({ kind: 'assistant_text', text: 'Hello world', streaming: true });
  });

  it('agent_message_delta creates item if missing', () => {
    const s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    const patch = reduceEvent(s, ev('t1', { type: 'agent_message_delta', delta: 'a' }));
    expect(patch.turns![0].items).toHaveLength(1);
  });

  it('agent_message finalizes streaming text', () => {
    const s: AgentState = {
      ...emptyState(),
      turns: [{ id: 't1', items: [{ kind: 'assistant_text', text: 'abc', streaming: true }], status: 'streaming' }],
    };
    const patch = reduceEvent(s, ev('t1', { type: 'agent_message', text: 'final' }));
    expect(patch.turns![0].items[0]).toMatchObject({ kind: 'assistant_text', text: 'final', streaming: false });
  });

  it('thinking_delta accumulates into thinking item', () => {
    let s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'thinking_delta', delta: 'Hmm' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'thinking_delta', delta: '...' })) };
    expect(s.turns[0].items[0]).toMatchObject({ kind: 'thinking', text: 'Hmm...' });
  });

  it('token_count updates tokens snapshot', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', {
        type: 'token_count',
        input_tokens: 1200,
        output_tokens: 300,
        cached_tokens: 200,
        cache_write_tokens: 500,
        total_tokens: 1500,
        cost_usd: 0.0123,
        provider: 'anthropic',
        credential_label: 'main-key',
      }),
    );
    expect(patch.tokens).toEqual({
      input: 1200,
      output: 300,
      cached: 200,
      cacheWrite: 500,
      total: 1500,
      cost: 0.0123,
      provider: 'anthropic',
      credentialLabel: 'main-key',
    });
  });

  it('token_count tolerates missing optional provider/credential_label', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', {
        type: 'token_count',
        input_tokens: 10,
        output_tokens: 5,
        cached_tokens: 0,
        cache_write_tokens: 0,
        total_tokens: 15,
      }),
    );
    expect(patch.tokens).toMatchObject({
      total: 15,
      cost: null,
      provider: null,
      credentialLabel: null,
      cacheWrite: 0,
    });
  });
});

// ====== Tool (2) ======

describe('reduceEvent — tool', () => {
  it('tool_call_begin adds running tool_call', () => {
    const s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    const patch = reduceEvent(
      s,
      ev('t1', { type: 'tool_call_begin', call_id: 'c1', tool_name: 'Bash', args: { cmd: 'ls' } }),
    );
    expect(patch.turns![0].items[0]).toMatchObject({
      kind: 'tool_call',
      toolName: 'Bash',
      callId: 'c1',
      status: 'running',
    });
  });

  it('tool_call_end marks tool as done and appends output', () => {
    let s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'tool_call_begin', call_id: 'c1', tool_name: 'Bash', args: {} })) };
    s = {
      ...s,
      ...reduceEvent(
        s,
        ev('t1', {
          type: 'tool_call_end',
          call_id: 'c1',
          output: { content: [{ type: 'text', text: 'ok' }], is_error: false, metadata: {}, elapsed_ms: 5 },
          is_error: false,
          elapsed_ms: 5,
        }),
      ),
    };
    const items = s.turns[0].items;
    const callItem = items.find((i) => i.kind === 'tool_call');
    expect(callItem && callItem.kind === 'tool_call' && callItem.status).toBe('done');
    const outItem = items.find((i) => i.kind === 'tool_output');
    // summarizeToolOutput 返回 JSON 序列化形式（ToolOutput 对象的字符串）。
    expect(outItem && outItem.kind === 'tool_output').toBeTruthy();
    if (outItem && outItem.kind === 'tool_output') {
      expect(outItem.text).toContain('"ok"');
    }
  });
});

// ====== 审批 / 提问 / 权限气泡 ======

describe('reduceEvent — approval / ask_user / bubble', () => {
  it('approval_request (tool kind) enqueues pending', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', {
        type: 'approval_request',
        request_id: 'a1',
        kind: { type: 'tool', tool_name: 'Bash', args: { cmd: 'rm -rf /' } },
      }),
    );
    expect(patch.pendingApprovals).toHaveLength(1);
    expect(patch.pendingApprovals![0]).toMatchObject({ id: 'a1', kind: 'tool', toolName: 'Bash' });
  });

  it('approval_request (plan kind) 不进 pendingApprovals —— plan 走独立的 pendingPlan', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', { type: 'approval_request', request_id: 'a2', kind: { type: 'plan', plan_id: 'p1', summary: 'refactor' } }),
    );
    // plan 审批走 plan_ready 事件 + PlanReadyModal,不进 tool/hook 的 pendingApprovals。
    expect(patch.pendingApprovals).toBeUndefined();
  });

  it('ask_user_question enqueues', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', { type: 'ask_user_question', request_id: 'q1', questions: [] }),
    );
    expect(patch.pendingQuestions).toHaveLength(1);
    expect(patch.pendingQuestions![0].id).toBe('q1');
  });

  it('ask_user_input enqueues', () => {
    const patch = reduceEvent(emptyState(), ev('t1', { type: 'ask_user_input', request_id: 'i1', prompt: 'why?' }));
    expect(patch.pendingAskUser).toHaveLength(1);
  });

  it('permission_bubble enqueues tool kind', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('t1', { type: 'permission_bubble', tool_name: 'Bash', risk: 'high' }),
    );
    expect(patch.pendingApprovals![0]).toMatchObject({ kind: 'tool', toolName: 'Bash' });
  });
});

// ====== Compaction / Error ======

describe('reduceEvent — compaction / error', () => {
  it('context_compacted appends summary', () => {
    const s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    const patch = reduceEvent(
      s,
      ev('t1', {
        type: 'context_compacted',
        strategy: 'llm_summarize',
        removed_messages: 5,
        before_tokens: 1000,
        after_tokens: 200,
      }),
    );
    const item = patch.turns![0].items[0];
    expect(item).toMatchObject({ kind: 'compacted' });
  });

  it('error (turn) appends formatted text', () => {
    const s: AgentState = { ...emptyState(), turns: [{ id: 't1', items: [], status: 'streaming' }] };
    const patch = reduceEvent(s, ev('t1', { type: 'error', code: 'E_FAIL', message: 'x' }));
    expect(patch.turns![0].items[0]).toMatchObject({ kind: 'error', text: 'E_FAIL: x' });
  });

  it('error (session) sets lastError', () => {
    const patch = reduceEvent(emptyState(), ev('', { type: 'error', code: 'E_FAIL', message: 'x' }));
    expect(patch.lastError).toBe('E_FAIL: x');
  });

  it('stream_error sets lastError with retry info', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'stream_error', code: '429', message: 'rate limited', retry_in_ms: 5000 }),
    );
    expect(patch.lastError).toContain('rate limited');
  });
});

// ====== Config / routing ======

describe('reduceEvent — config / routing', () => {
  it('config_reloaded updates timestamp', () => {
    const before = Date.now();
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'config_reloaded', path: '/cfg', sections_changed: ['anthropic'], at: 0 }),
    );
    expect(patch.configReloadedAt).toBeGreaterThanOrEqual(before);
  });

  it('routing stores snapshot', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', {
        type: 'routing',
        kind: 'switched',
        role: 'main',
        from_credential: 'work',
        to_credential: 'personal',
        reason: 'rate_limited',
      }),
    );
    expect(patch.lastRouting).toMatchObject({ kind: 'switched', from: 'work', to: 'personal' });
  });
});

// ====== Collab (3) ======

describe('reduceEvent — collab', () => {
  it('collab_started creates session', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'collab_started', id: 'd1', participants: ['a', 'b'], mode: 'sequential' }),
    );
    expect(patch.collabSessions).toHaveLength(1);
    expect(patch.collabSessions![0].id).toBe('d1');
  });

  it('collab_message appends to existing session', () => {
    let s: AgentState = emptyState();
    s = { ...s, ...reduceEvent(s, ev('', { type: 'collab_started', id: 'd1', participants: ['a'], mode: 'seq' })) };
    s = {
      ...s,
      ...reduceEvent(
        s,
        ev('', { type: 'collab_message', id: 'd1', from: 'a', kind: 'utterance', content: 'hi', round: 0 }),
      ),
    };
    expect(s.collabSessions[0].messages).toHaveLength(1);
  });

  it('collab_finished marks session done', () => {
    let s: AgentState = emptyState();
    s = { ...s, ...reduceEvent(s, ev('', { type: 'collab_started', id: 'd1', participants: [], mode: 'seq' })) };
    s = { ...s, ...reduceEvent(s, ev('', { type: 'collab_finished', id: 'd1', outcome: 'consensus', rounds: 3 })) };
    expect(s.collabSessions[0].status).toBe('done');
    expect(s.collabSessions[0].rounds).toBe(3);
  });
});

// ====== MCP (3) ======

describe('reduceEvent — MCP', () => {
  it('mcp_server_started registers server', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'mcp_server_started', server: 'fs', tool_count: 3, transport: 'stdio' }),
    );
    expect(patch.mcpServers![0]).toMatchObject({ name: 'fs', status: 'started' });
  });

  it('mcp_server_failed marks server as failed', () => {
    let s: AgentState = emptyState();
    s = { ...s, ...reduceEvent(s, ev('', { type: 'mcp_server_started', server: 'fs', tool_count: 0, transport: 'stdio' })) };
    s = { ...s, ...reduceEvent(s, ev('', { type: 'mcp_server_failed', server: 'fs', error: 'died', will_retry: false })) };
    expect(s.mcpServers[0].status).toBe('failed');
  });

  it('mcp_tool_invoked appends to invocations', () => {
    const patch = reduceEvent(emptyState(), ev('t1', { type: 'mcp_tool_invoked', server: 'fs', tool: 'read', call_id: 'c1' }));
    expect(patch.mcpInvocations).toHaveLength(1);
    expect(patch.mcpInvocations![0]).toMatchObject({ server: 'fs', tool: 'read' });
  });
});

// ====== LSP (2) ======

describe('reduceEvent — LSP', () => {
  it('lsp_server_started registers', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'lsp_server_started', server: 'rust-analyzer', methods: ['textDocument/hover'], language_ids: ['rust'] }),
    );
    expect(patch.lspServers![0]).toMatchObject({ name: 'rust-analyzer', status: 'started' });
  });

  it('lsp_server_failed marks failed', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'lsp_server_failed', server: 'rust-analyzer', error: 'no binary', will_retry: false }),
    );
    expect(patch.lspServers![0].status).toBe('failed');
  });
});

// ====== Plan mode (5) ======

describe('reduceEvent — plan mode', () => {
  it('plan_request sets pendingPlan', () => {
    const patch = reduceEvent(emptyState(), ev('t1', { type: 'plan_request', task: 'refactor auth' }));
    expect(patch.pendingPlan?.id).toBe('refactor auth');
  });

  it('plan_ready sets pendingPlan with plan_id', () => {
    const patch = reduceEvent(emptyState(), ev('t1', { type: 'plan_ready', plan_id: 'p1', markdown: '...' }));
    expect(patch.pendingPlan?.id).toBe('p1');
  });

  it('plan_approved clears pendingPlan', () => {
    let s: AgentState = emptyState();
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'plan_ready', plan_id: 'p1', markdown: '' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'plan_approved', plan_id: 'p1' })) };
    expect(s.pendingPlan).toBeNull();
  });

  it('plan_rejected clears pendingPlan', () => {
    let s: AgentState = emptyState();
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'plan_ready', plan_id: 'p1', markdown: '' })) };
    s = { ...s, ...reduceEvent(s, ev('t1', { type: 'plan_rejected', plan_id: 'p1', reason: 'no' })) };
    expect(s.pendingPlan).toBeNull();
  });

  it('permission_mode_changed updates to new mode', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', { type: 'permission_mode_changed', from: 'auto', to: 'plan' }),
    );
    expect(patch.permissionMode).toBe('plan');
  });
});

// ====== Plugin / quota ======

describe('reduceEvent — plugin / quota', () => {
  it('quota_exhausted 推 toast warn 告知用户配额耗尽', () => {
    const state = emptyState();
    const patch = reduceEvent(
      state,
      ev('', {
        type: 'quota_exhausted',
        provider: 'anthropic',
        label: 'plan-a',
        used_tokens: 50000,
        max_tokens: 50000,
        window_ends_secs: 1800,
      }),
    );
    expect(patch.toasts).toHaveLength(1);
    expect(patch.toasts![0].kind).toBe('warn');
    expect(patch.toasts![0].message).toContain('anthropic/plan-a');
    expect(patch.toasts![0].message).toContain('50000/50000');
    expect(patch.toasts![0].message).toContain('30min');
    expect(patch.toasts![0].ttlMs).toBe(6000);
  });

  it('plan_step 是 no-op(待接入 plan 步骤 UI)', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', {
        type: 'plan_step',
        plan_id: 'p1',
        index: 0,
        total: 3,
        status: 'in_progress',
        title: 'analyze',
      }),
    );
    expect(patch).toEqual({});
  });

  it('plugin_loaded 是 no-op(待接入 plugins panel)', () => {
    const patch = reduceEvent(
      emptyState(),
      ev('', {
        type: 'plugin_loaded',
        plugin: 'demo',
        scope: 'user',
        version: '1.0',
        skill_count: 2,
      }),
    );
    expect(patch).toEqual({});
  });
});
