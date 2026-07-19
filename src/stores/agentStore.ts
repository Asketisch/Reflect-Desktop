/**
 * agentStore —— 全局 agent 状态(Zustand)。
 *
 * ## 为什么需要它
 *
 * M1.x 的 `useAgent()` 用本地 `useState`,导致 `MessageList` / `Composer` /
 * `HomeView` / `DebugView` / `StatusBar` 各持**独立**的 turns 副本 ——
 * 一个组件 submit 后,其它组件看不到新消息。本 store 修复这个结构 bug:
 * 单一 state,单一事件订阅(在 AppProviders mount 时建立一次),所有组件共享。
 *
 * ## Turn 模型
 *
 * 旧 `Turn = { user, reply, done }` 把 33 种事件压扁成一个字符串,
 * 工具调用/思考/审批/计划/错误全丢。新 `Turn = { id, items, status }`,
 * `TurnItem` 是 discriminated union,每个 item 对应一个或多个 `EventMsg`:
 *   - user_text       (submit 时乐观追加)
 *   - assistant_text  (AgentMessage 最终文本)
 *   - assistant_delta (AgentMessageDelta 流式片段 —— 渲染时拼接到同 turn 的 streaming item)
 *   - thinking        (ThinkingDelta)
 *   - tool_call       (ToolCallBegin + 配对的 ToolCallEnd)
 *   - tool_output     (ToolCallEnd 的输出)
 *   - error           (Error / StreamError)
 *   - compacted       (ContextCompacted)
 *
 * 不在 turn 内的事件(session 级)进 store 的顶层状态:
 *   - session_configured → session
 *   - approval_request / ask_user_question / ask_user_input / plan_request → pending 队列
 *   - mcp_server_started/failed / lsp_server_started/failed → mcp/lsp 列表
 *   - permission_mode_changed / config_reloaded → 标志位
 *
 * `event.id` 为 `EVENT_ID_NONE`("") 的事件是 session 级(无匹配 submission),
 * 不挂到任何 turn。
 */
import { create } from 'zustand';
import {
  reflect_submit,
  reflect_interrupt,
  reflect_compact,
  reflect_rewind,
  reflect_shutdown,
  reflect_tool_approval,
  reflect_hook_approval,
  reflect_plan_approval,
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_cycle_permission_mode,
  reflect_ask_user_question_response,
  reflect_ask_user_input_response,
  onReflectEvent,
} from '@/utils/tauri';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';
import type { ReviewDecision } from '@/utils/types';

// ====== 常量 ======

/** 空字符串 = session 级事件(无匹配 submission)。对齐 Rust EVENT_ID_NONE。 */
const EVENT_ID_NONE = '';

// ====== Turn 数据模型 ======

/** 单个 turn 内的一行(item)。 */
export type TurnItem =
  | { kind: 'user_text'; text: string }
  | { kind: 'assistant_text'; text: string; streaming?: boolean }
  | { kind: 'thinking'; text: string }
  | { kind: 'tool_call'; toolName: string; argsSummary: string; callId: string; status: 'running' | 'done' | 'error' }
  | { kind: 'tool_output'; callId: string; text: string; isError: boolean }
  | { kind: 'error'; text: string }
  | { kind: 'compacted'; summary: string };

export type TurnStatus = 'streaming' | 'done' | 'aborted';

export interface Turn {
  id: string;
  items: TurnItem[];
  status: TurnStatus;
}

// ====== Session 级 pending 状态 ======

/** 待审批的工具/计划/hook 调用。 */
export interface PendingApproval {
  id: string;
  kind: 'tool' | 'hook' | 'plan';
  toolName?: string;
  argsSummary?: string;
  /** 对应的 turn id(可能为空字符串)。 */
  turnId: string;
}

/** 待回答的结构化问题(AskUserQuestion)。 */
export interface PendingQuestion {
  id: string;
  /** 原始 payload,UI 自行解析 questions/options。 */
  payload: unknown;
  turnId: string;
}

/** 待回答的自由文本输入(AskUserInput)。 */
export interface PendingAskUser {
  id: string;
  payload: unknown;
  turnId: string;
}

/** 待审批的计划(PlanReady)。 */
export interface PendingPlan {
  id: string;
  payload: unknown;
  turnId: string;
}

// ====== MCP / LSP server 状态 ======

export interface McpServerEntry {
  name: string;
  status: 'started' | 'failed';
  detail?: string;
}

export interface LspServerEntry {
  name: string;
  status: 'started' | 'failed';
  detail?: string;
}

// ====== Session 信息 ======

export interface AgentSession {
  model: string;
  provider: string;
}

// ====== Store state ======

export interface AgentState {
  turns: Turn[];
  session: AgentSession | null;
  /** 当前 permission mode('auto'|'prompt'|'deny'|'plan')。 */
  permissionMode: string;
  /** pending 队列(供 modals 消费)。 */
  pendingApprovals: PendingApproval[];
  pendingQuestions: PendingQuestion[];
  pendingAskUser: PendingAskUser[];
  pendingPlan: PendingPlan | null;
  /** MCP / LSP server 运行时状态。 */
  mcpServers: McpServerEntry[];
  lspServers: LspServerEntry[];
  /** 最近一次错误(顶层 banner 用)。 */
  lastError: string | null;
  /** 事件订阅是否已建立(防止重复订阅)。 */
  subscribed: boolean;

  // ====== Actions ======
  /** 建立 reflect_event 订阅(幂等,AppProviders mount 时调一次)。 */
  subscribe: () => () => void;
  /** 提交用户文本输入。 */
  submit: (text: string) => Promise<void>;
  /** 中断当前 turn。 */
  interrupt: () => Promise<void>;
  /** 紧凑化上下文。 */
  compact: () => Promise<void>;
  /** 回退到指定 turn。 */
  rewind: (toTurnId?: string) => Promise<void>;
  /** 关闭 agent。 */
  shutdown: () => Promise<void>;
  /** 审批(tool/hook/plan)。 */
  approve: (kind: 'tool' | 'hook' | 'plan', id: string, decision: ReviewDecision) => Promise<void>;
  /** 进入/退出 plan 模式。 */
  enterPlanMode: (task: string) => Promise<void>;
  exitPlanMode: () => Promise<void>;
  /** 设置 reasoning effort。 */
  setEffort: (level: 'low' | 'medium' | 'high') => Promise<void>;
  /** 设置/循环 permission mode。 */
  setPermissionMode: (mode: string) => Promise<void>;
  cyclePermissionMode: () => Promise<void>;
  /** 回答 AskUserQuestion / AskUserInput。 */
  answerQuestion: (id: string, answers: unknown) => Promise<void>;
  answerInput: (id: string, text: string) => Promise<void>;
  /** 清除错误。 */
  clearError: () => void;
  /** 测试/重置用:清空所有 turns + pending。 */
  reset: () => void;
}

// ====== UUID(与旧 useAgent 一致) ======

const uuid = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

// ====== Redducer:把 ReflectEvent 应用到 state ======

/**
 * 纯函数 —— 根据 event 更新 state。导出供测试。
 *
 * 不直接 mutate;返回新数组/对象。Zustand 的 set((s) => ...) 会用返回值。
 */
export function reduceEvent(state: AgentState, e: ReflectEvent): Partial<AgentState> {
  const { msg } = e;
  const turnId = e.id;
  const isSessionEvent = turnId === EVENT_ID_NONE;

  switch (msg.type) {
    case 'session_configured': {
      const model = String(msg.model ?? '');
      const provider = String(msg.provider ?? '');
      if (model) return { session: { model, provider } };
      return {};
    }

    case 'turn_started': {
      // 新 turn 开始 —— 确保有一个空 turn(若 submit 乐观加了就不重复)。
      const exists = state.turns.some((t) => t.id === turnId);
      if (exists) return {};
      return {
        turns: [...state.turns, { id: turnId, items: [], status: 'streaming' }],
      };
    }

    case 'agent_message_delta': {
      const delta = String(msg.delta ?? '');
      if (!delta) return {};
      return { turns: upsertDelta(state.turns, turnId, delta) };
    }

    case 'agent_message': {
      const text = String(msg.text ?? '');
      return { turns: finalizeAssistantText(state.turns, turnId, text) };
    }

    case 'thinking_delta': {
      const delta = String(msg.delta ?? '');
      if (!delta) return {};
      return { turns: upsertThinking(state.turns, turnId, delta) };
    }

    case 'tool_call_begin': {
      const toolName = String(msg.tool_name ?? msg.name ?? '');
      const callId = String(msg.call_id ?? msg.id ?? '');
      const argsSummary = summarizeArgs(msg.args);
      return {
        turns: appendItem(state.turns, turnId, {
          kind: 'tool_call',
          toolName,
          callId,
          argsSummary,
          status: 'running',
        }),
      };
    }

    case 'tool_call_end': {
      const callId = String(msg.call_id ?? msg.id ?? '');
      const outputText = summarizeToolOutput(msg.output ?? msg.result);
      const isError = Boolean(msg.is_error ?? msg.error);
      return {
        turns: appendItem(state.turns, turnId, {
          kind: 'tool_output',
          callId,
          text: outputText,
          isError,
        }).map((t) => ({
          ...t,
          items: t.items.map((it) =>
            it.kind === 'tool_call' && it.callId === callId
              ? { ...it, status: isError ? ('error' as const) : ('done' as const) }
              : it,
          ),
        })),
      };
    }

    case 'error':
    case 'stream_error': {
      const text = String(msg.message ?? msg.text ?? msg.error ?? 'unknown error');
      if (isSessionEvent) {
        return { lastError: text };
      }
      return { turns: appendItem(state.turns, turnId, { kind: 'error', text }) };
    }

    case 'context_compacted': {
      const summary = String(msg.summary ?? 'context compacted');
      return { turns: appendItem(state.turns, turnId, { kind: 'compacted', summary }) };
    }

    case 'turn_complete': {
      return { turns: markTurn(state.turns, turnId, 'done') };
    }

    case 'turn_aborted': {
      return { turns: markTurn(state.turns, turnId, 'aborted') };
    }

    case 'approval_request': {
      const id = String(msg.id ?? msg.call_id ?? '');
      const kind = (String(msg.kind ?? 'tool') as 'tool' | 'hook' | 'plan');
      const approval: PendingApproval = {
        id,
        kind,
        toolName: msg.tool_name ? String(msg.tool_name) : undefined,
        argsSummary: msg.args ? summarizeArgs(msg.args) : undefined,
        turnId,
      };
      return { pendingApprovals: [...state.pendingApprovals, approval] };
    }

    case 'permission_bubble': {
      // permission_bubble 与 approval_request 类似,统一进 approvals 队列。
      const id = String(msg.id ?? 'permission');
      const approval: PendingApproval = {
        id,
        kind: 'tool',
        turnId,
      };
      return { pendingApprovals: [...state.pendingApprovals, approval] };
    }

    case 'ask_user_question': {
      const id = String(msg.id ?? msg.request_id ?? '');
      return {
        pendingQuestions: [...state.pendingQuestions, { id, payload: msg, turnId }],
      };
    }

    case 'ask_user_input': {
      const id = String(msg.id ?? msg.request_id ?? '');
      return {
        pendingAskUser: [...state.pendingAskUser, { id, payload: msg, turnId }],
      };
    }

    case 'plan_request':
    case 'plan_ready': {
      const id = String(msg.id ?? msg.plan_id ?? '');
      return { pendingPlan: { id, payload: msg, turnId } };
    }

    case 'plan_approved':
    case 'plan_rejected': {
      return { pendingPlan: null };
    }

    case 'permission_mode_changed': {
      const mode = String(msg.mode ?? '');
      return { permissionMode: mode || state.permissionMode };
    }

    case 'mcp_server_started': {
      const name = String(msg.name ?? msg.server ?? '');
      if (!name) return {};
      return { mcpServers: upsertServer(state.mcpServers, name, 'started') };
    }

    case 'mcp_server_failed': {
      const name = String(msg.name ?? msg.server ?? '');
      if (!name) return {};
      return {
        mcpServers: upsertServer(state.mcpServers, name, 'failed', msg.error ? String(msg.error) : undefined),
      };
    }

    case 'lsp_server_started': {
      const name = String(msg.name ?? msg.server ?? '');
      if (!name) return {};
      return { lspServers: upsertServer(state.lspServers, name, 'started') };
    }

    case 'lsp_server_failed': {
      const name = String(msg.name ?? msg.server ?? '');
      if (!name) return {};
      return {
        lspServers: upsertServer(state.lspServers, name, 'failed', msg.error ? String(msg.error) : undefined),
      };
    }

    default:
      // 未处理的事件类型(turn_rewound / shutdown_complete / token_count /
      // config_reloaded / routing / collab_* / mcp_tool_invoked / plan_step /
      // plugin_loaded / quota_exhausted)—— 静默,后续按需补。
      return {};
  }
}

// ====== Reducer 辅助 ======

/** 追加 item 到指定 turn(若 turn 不存在则创建)。 */
function appendItem(turns: Turn[], turnId: string, item: TurnItem): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  const idx = turns.findIndex((t) => t.id === turnId);
  if (idx === -1) {
    return [...turns, { id: turnId, items: [item], status: 'streaming' as const }];
  }
  return turns.map((t, i) => (i === idx ? { ...t, items: [...t.items, item] } : t));
}

/** 标记 turn 状态。 */
function markTurn(turns: Turn[], turnId: string, status: TurnStatus): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  return turns.map((t) => (t.id === turnId ? { ...t, status } : t));
}

/** 把 delta 追加到 turn 的最后一个 assistant_text item(或新建)。 */
function upsertDelta(turns: Turn[], turnId: string, delta: string): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (it.kind === 'assistant_text') {
        items[i] = { ...it, text: it.text + delta, streaming: true };
        return { ...turn, items };
      }
    }
    // 没有现成的 assistant_text —— 新建一个。
    items.push({ kind: 'assistant_text', text: delta, streaming: true });
    return { ...turn, items };
  });
}

/** 把 thinking delta 追加到 turn 的最后一个 thinking item(或新建)。 */
function upsertThinking(turns: Turn[], turnId: string, delta: string): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (it.kind === 'thinking') {
        items[i] = { ...it, text: it.text + delta };
        return { ...turn, items };
      }
    }
    items.push({ kind: 'thinking', text: delta });
    return { ...turn, items };
  });
}

/** AgentMessage 最终文本:替换 streaming delta 为最终值。 */
function finalizeAssistantText(turns: Turn[], turnId: string, text: string): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    // 替换最后一个 assistant_text。
    for (let i = items.length - 1; i >= 0; i--) {
      if (items[i].kind === 'assistant_text') {
        items[i] = { kind: 'assistant_text', text, streaming: false };
        return { ...turn, items };
      }
    }
    // 没有现成的 —— 直接加最终文本。
    items.push({ kind: 'assistant_text', text, streaming: false });
    return { ...turn, items };
  });
}

/** 对单个 turn 应用 fn;turn 不存在则创建(空)。 */
function mapTurn(turns: Turn[], turnId: string, fn: (t: Turn) => Turn): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  const idx = turns.findIndex((t) => t.id === turnId);
  if (idx === -1) {
    return [...turns, fn({ id: turnId, items: [], status: 'streaming' })];
  }
  return turns.map((t, i) => (i === idx ? fn(t) : t));
}

/** upsert MCP/LSP server 条目。 */
function upsertServer<T extends McpServerEntry | LspServerEntry>(
  list: T[],
  name: string,
  status: 'started' | 'failed',
  detail?: string,
): T[] {
  const idx = list.findIndex((s) => s.name === name);
  if (idx === -1) return [...list, { name, status, detail } as T];
  return list.map((s, i) => (i === idx ? { ...s, status, detail } : s));
}

/** 把 tool args 序列化成单行摘要。 */
function summarizeArgs(args: unknown): string {
  if (args == null) return '';
  try {
    const s = typeof args === 'string' ? args : JSON.stringify(args);
    return s.length > 200 ? s.slice(0, 200) + '…' : s;
  } catch {
    return String(args);
  }
}

/** 把 tool output 序列化成可显示文本。 */
function summarizeToolOutput(output: unknown): string {
  if (output == null) return '';
  if (typeof output === 'string') return output;
  try {
    return JSON.stringify(output, null, 2);
  } catch {
    return String(output);
  }
}

// ====== Store 实现 ======

export const useAgentStore = create<AgentState>((set, get) => ({
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

  subscribe: () => {
    if (get().subscribed) {
      // 已订阅 —— 返回 noop(防止 AppProviders 重 mount 时重复订阅)。
      return () => {};
    }
    set({ subscribed: true });
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void (async () => {
      const fn = await onReflectEvent((e: ReflectEvent) => {
        const patch = reduceEvent(get(), e);
        if (Object.keys(patch).length > 0) {
          set(patch);
        }
      });
      if (cancelled) {
        fn?.();
      } else {
        unlisten = fn;
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
      set({ subscribed: false });
    };
  },

  submit: async (text: string) => {
    const id = uuid();
    const submission: ReflectSubmission = {
      id,
      op: { type: 'user_input', items: [{ type: 'text', text }] },
    };
    // 乐观:立即加一个带 user_text 的 turn。
    set((s) => ({
      turns: [...s.turns, { id, items: [{ kind: 'user_text', text }], status: 'streaming' }],
    }));
    await reflect_submit(submission);
  },

  interrupt: async () => {
    await reflect_interrupt();
  },

  compact: async () => {
    await reflect_compact();
  },

  rewind: async (toTurnId?: string) => {
    await reflect_rewind(toTurnId);
  },

  shutdown: async () => {
    await reflect_shutdown();
  },

  approve: async (kind, id, decision) => {
    // 乐观:先从 pending 队列移除(即时关闭 modal),再 await IPC。
    set((s) => ({ pendingApprovals: s.pendingApprovals.filter((a) => a.id !== id) }));
    if (kind === 'tool') await reflect_tool_approval(id, decision);
    else if (kind === 'hook') await reflect_hook_approval(id, decision);
    else await reflect_plan_approval(id, decision);
  },

  enterPlanMode: async (task: string) => {
    await reflect_enter_plan_mode(task);
  },

  exitPlanMode: async () => {
    await reflect_exit_plan_mode();
  },

  setEffort: async (level: 'low' | 'medium' | 'high') => {
    await reflect_set_effort(level);
  },

  setPermissionMode: async (mode: string) => {
    await reflect_set_permission_mode(mode);
  },

  cyclePermissionMode: async () => {
    await reflect_cycle_permission_mode();
  },

  answerQuestion: async (id: string, answers: unknown) => {
    set((s) => ({ pendingQuestions: s.pendingQuestions.filter((q) => q.id !== id) }));
    await reflect_ask_user_question_response(id, answers);
  },

  answerInput: async (id: string, text: string) => {
    set((s) => ({ pendingAskUser: s.pendingAskUser.filter((q) => q.id !== id) }));
    await reflect_ask_user_input_response(id, text);
  },

  clearError: () => set({ lastError: null }),

  reset: () =>
    set({
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
    }),
}));

// ====== 兼容 hook:让现有 useAgent() 消费方平滑迁移 ======

/**
 * useAgent —— 兼容旧 API。内部从 store 读,保证所有组件共享同一 state。
 *
 * 旧消费方(MessageList/Composer/HomeView/DebugView/StatusBar)用的
 * `{ turns, session, submit }` 形态保持不变,但 turns 现在是富 Turn[]。
 *
 * 新代码应直接用 `useAgentStore()` 的 selector,避免 re-render 过宽。
 */
export function useAgent() {
  const turns = useAgentStore((s) => s.turns);
  const session = useAgentStore((s) => s.session);
  const submit = useAgentStore((s) => s.submit);
  return { turns, session, submit };
}
