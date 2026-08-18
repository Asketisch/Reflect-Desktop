import type { ReflectEvent } from '@/types/protocol';
import type { AgentState, PendingApproval } from './types';
import { upsertServer } from './servers';
import {
  appendItem,
  EVENT_ID_NONE,
  finalizeAssistantText,
  markTurn,
  summarizeArgs,
  summarizeToolOutput,
  upsertDelta,
  upsertThinking,
} from './turns';

export function reduceEvent(state: AgentState, event: ReflectEvent): Partial<AgentState> {
  const { msg } = event;
  const turnId = event.id;
  const isSessionEvent = turnId === EVENT_ID_NONE;

  switch (msg.type) {
    case 'session_configured': {
      const patch: Partial<AgentState> = {};
      if (msg.model) patch.session = { model: msg.model, provider: msg.provider };
      if (typeof msg.context_window_size === 'number') {
        patch.contextWindowSize = msg.context_window_size;
      }
      return patch;
    }

    case 'turn_started':
      if (state.turns.some((turn) => turn.id === turnId)) return {};
      return { turns: [...state.turns, { id: turnId, items: [], status: 'streaming' }] };

    case 'turn_complete':
      return { turns: markTurn(state.turns, turnId, 'done') };

    case 'turn_aborted':
      return { turns: markTurn(state.turns, turnId, 'aborted') };

    case 'turn_rewound': {
      const cutoff = msg.to_turn_id;
      if (!cutoff) return {};
      // 按 id 查找截断 turn;保留所有该 turn 及之前的 turn。
      // v4 UUID 无法字典序比较,改为查找索引。若 id 未知则保持 turns 不动
      // (后端是单一事实源,下一次回放会重新对齐)。
      const idx = state.turns.findIndex((turn) => turn.id === cutoff);
      if (idx < 0) return {};
      return { turns: state.turns.slice(0, idx + 1) };
    }

    case 'shutdown_complete':
      return { lastError: 'agent shut down' };

    case 'agent_message_delta':
      return msg.delta ? { turns: upsertDelta(state.turns, turnId, msg.delta) } : {};

    case 'agent_message':
      return { turns: finalizeAssistantText(state.turns, turnId, msg.text) };

    case 'thinking_delta':
      return msg.delta ? { turns: upsertThinking(state.turns, turnId, msg.delta) } : {};

    case 'token_count':
      return {
        tokens: {
          input: msg.input_tokens,
          output: msg.output_tokens,
          cached: msg.cached_tokens,
          cacheWrite: msg.cache_write_tokens,
          total: msg.total_tokens,
          cost: msg.cost_usd ?? null,
          provider: msg.provider ?? null,
          credentialLabel: msg.credential_label ?? null,
        },
      };

    case 'tool_call_begin':
      return {
        turns: appendItem(state.turns, turnId, {
          kind: 'tool_call',
          toolName: msg.tool_name,
          callId: msg.call_id,
          argsSummary: summarizeArgs(msg.args),
          status: 'running',
        }),
      };

    case 'tool_call_end': {
      const withOutput = appendItem(state.turns, turnId, {
        kind: 'tool_output',
        callId: msg.call_id,
        text: summarizeToolOutput(msg.output),
        isError: msg.is_error,
      });
      // 只重写拥有对应 tool_call 的 turn,其他 turn 重写毫无意义。
      const targetTurnIdx = withOutput.findIndex((t) => t.id === turnId);
      if (targetTurnIdx < 0) return { turns: withOutput };
      const target = withOutput[targetTurnIdx];
      const nextStatus: 'done' | 'error' = msg.is_error ? 'error' : 'done';
      const updatedTurn = {
        ...target,
        items: target.items.map((item) =>
          item.kind === 'tool_call' && item.callId === msg.call_id
            ? { ...item, status: nextStatus }
            : item,
        ),
      };
      const turns = withOutput.slice();
      turns[targetTurnIdx] = updatedTurn;
      return { turns };
    }

    case 'approval_request': {
      // plan 类型的审批走独立的 plan_ready 事件 + pendingPlan + PlanReadyModal,
      // 不进 pendingApprovals(后者只收 tool / hook)。
      if (msg.kind.type === 'plan') return {};
      const approval: PendingApproval = {
        id: msg.request_id,
        kind: msg.kind.type,
        toolName: msg.kind.type === 'tool' ? msg.kind.tool_name : undefined,
        argsSummary: msg.kind.type === 'tool' ? summarizeArgs(msg.kind.args) : undefined,
        turnId,
      };
      return { pendingApprovals: [...state.pendingApprovals, approval] };
    }

    case 'ask_user_question':
      return {
        pendingQuestions: [
          ...state.pendingQuestions,
          { id: msg.request_id, payload: msg, turnId },
        ],
      };

    case 'ask_user_input':
      return {
        pendingAskUser: [
          ...state.pendingAskUser,
          { id: msg.request_id, payload: msg, turnId },
        ],
      };

    case 'permission_bubble':
      return {
        pendingApprovals: [
          ...state.pendingApprovals,
          {
            id: `bubble-${msg.tool_name}-${turnId}`,
            kind: 'tool',
            toolName: msg.tool_name,
            risk: msg.risk,
            turnId,
          },
        ],
      };

    case 'context_compacted':
      return {
        turns: appendItem(state.turns, turnId, {
          kind: 'compacted',
          summary: `${msg.strategy}: ${msg.removed_messages} msgs (${msg.before_tokens} → ${msg.after_tokens} tokens)`,
        }),
      };

    case 'error':
      return isSessionEvent
        ? { lastError: `${msg.code}: ${msg.message}` }
        : {
            turns: appendItem(state.turns, turnId, {
              kind: 'error',
              text: `${msg.code}: ${msg.message}`,
            }),
          };

    case 'stream_error': {
      const text =
        msg.message || `stream error ${msg.code} (retry in ${msg.retry_in_ms}ms)`;
      return isSessionEvent
        ? { lastError: text }
        : { turns: appendItem(state.turns, turnId, { kind: 'error', text }) };
    }

    case 'config_reloaded':
      return { configReloadedAt: Date.now() };

    case 'routing':
      return {
        lastRouting: {
          kind: msg.kind,
          role: msg.role,
          from: msg.from_credential,
          to: msg.to_credential,
          reason: msg.reason,
        },
      };

    case 'collab_started':
      return {
        collabSessions: [
          ...state.collabSessions,
          {
            id: msg.id,
            participants: msg.participants,
            mode: msg.mode,
            startedAt: Date.now(),
            status: 'running',
            messages: [],
          },
        ],
      };

    case 'collab_message':
      return {
        collabSessions: state.collabSessions.map((session) =>
          session.id === msg.id
            ? {
                ...session,
                messages: [
                  ...session.messages,
                  {
                    from: msg.from,
                    kind: msg.kind,
                    content: msg.content,
                    round: msg.round,
                    at: Date.now(),
                  },
                ],
              }
            : session,
        ),
      };

    case 'collab_finished':
      return {
        collabSessions: state.collabSessions.map((session) =>
          session.id === msg.id
            ? {
                ...session,
                status: 'done' as const,
                outcome: msg.outcome,
                rounds: msg.rounds,
              }
            : session,
        ),
      };

    case 'mcp_server_started':
      return {
        mcpServers: upsertServer(
          state.mcpServers,
          msg.server,
          'started',
          `${msg.tool_count} tools`,
        ),
      };

    case 'mcp_server_failed':
      return {
        mcpServers: upsertServer(state.mcpServers, msg.server, 'failed', msg.error),
      };

    case 'mcp_tool_invoked':
      return {
        mcpInvocations: [
          ...state.mcpInvocations,
          { server: msg.server, tool: msg.tool, callId: msg.call_id, at: Date.now() },
        ].slice(-50),
      };

    case 'lsp_server_started':
      return {
        lspServers: upsertServer(
          state.lspServers,
          msg.server,
          'started',
          `${msg.methods.length} methods`,
        ),
      };

    case 'lsp_server_failed':
      return {
        lspServers: upsertServer(state.lspServers, msg.server, 'failed', msg.error),
      };

    case 'plan_request':
      return { pendingPlan: { id: msg.task, payload: msg, turnId } };

    case 'plan_ready':
      return { pendingPlan: { id: msg.plan_id, payload: msg, turnId } };

    case 'plan_approved':
    case 'plan_rejected':
      return { pendingPlan: null };

    case 'plan_step':
      // plan 执行进度。TODO:接入 plan 步骤 UI(目前仅补全类型,不渲染)。
      return {};

    case 'permission_mode_changed':
      return { permissionMode: msg.to };

    case 'plugin_loaded':
      // 插件运行时挂载完成。TODO:展示到 plugins panel;目前仅补全类型。
      return {};

    case 'quota_exhausted': {
      // 配额耗尽 → 立即 toast warn,告知用户 agent 因此停止。
      const usedStr = msg.used_tokens && msg.max_tokens
        ? ` (${msg.used_tokens}/${msg.max_tokens} tokens)`
        : '';
      const cooldownStr = msg.window_ends_secs
        ? ` — ${Math.ceil(msg.window_ends_secs / 60)}min 后重置`
        : '';
      const toast: AgentState['toasts'][number] = {
        id: `quota-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        kind: 'warn',
        message: `配额耗尽:${msg.provider}/${msg.label}${usedStr}${cooldownStr}`,
        ttlMs: 6000,
        createdAt: Date.now(),
      };
      return { toasts: [...state.toasts, toast] };
    }
  }
}
