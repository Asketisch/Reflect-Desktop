import { create } from 'zustand';
import { subscribeAgentEvent } from '@/services/agentEventBus';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';
import {
  reflect_ask_user_input_response,
  reflect_ask_user_question_response,
  reflect_compact,
  reflect_cycle_permission_mode,
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  reflect_get_config,
  reflect_hook_approval,
  reflect_interrupt,
  reflect_plan_approval,
  reflect_rewind,
  reflect_save_config,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_shutdown,
  reflect_submit,
  reflect_tool_approval,
} from '@/utils/commands';
import { turnsFromRollout } from '../replay';
import {
  createAutoCompactController,
} from './autoCompact';
import {
  createPlanFailoverController,
  extractExhaustionSignal,
  isAutoFailoverEnabled,
} from './planFailover';
import { reduceEvent } from './reducer';
import { markTurn } from './turns';
import { selectHasPendingInteraction, selectIsTurnRunning } from './selectors';
import { createToastActions, uuid } from './toast';
import type { AgentState } from './types';

export const useAgentStore = create<AgentState>((set, get) => {
  // 按 plan 最大上下文自动压缩控制器(内部有开关/节流/触发闩)。
  const autoCompact = createAutoCompactController({
    getConfig: () => reflect_get_config(),
    compact: () => reflect_compact(),
  });
  // coding plan 耗尽自动切换控制器:闭包持有 get/set,防抖状态随 store 生命周期。
  // getConfig/saveConfig 用箭头包装延迟解引用 —— 测试环境部分 mock
  // @/utils/commands 时不至于在 store 构造期就触碰未导出的名字。
  const planFailover = createPlanFailoverController({
    getConfig: () => reflect_get_config(),
    saveConfig: (toml) => reflect_save_config(toml),
    pushToast: (input) => {
      get().pushToast(input);
    },
    isEnabled: isAutoFailoverEnabled,
    onSwitched: (from, to) => set({ planFailover: { from, to, at: Date.now() } }),
    currentProvider: () => get().session?.provider ?? null,
  });

  return {
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
  loadedSessionId: null,
  hydrateSession: (id, records) => {
    const { turns, compactions } = turnsFromRollout(records);
    set({ turns, compactions, loadedSessionId: id, lastError: null, queuedMessages: [] });
  },
  clearSession: () =>
    set({
      turns: [],
      loadedSessionId: null,
      pendingApprovals: [],
      pendingQuestions: [],
      pendingAskUser: [],
      pendingPlan: null,
      queuedMessages: [],
      // 会话域的遥测/路由数据一并重置:回到首页(或新会话首事件到来前)
      // 不应展示上一个会话的 token/压缩/路由;planFailover 的
      // currentProvider 兜底也不应读到旧会话的 provider。
      session: null,
      tokens: null,
      contextWindowSize: null,
      compactions: { count: 0, removedMessages: 0, tokensSaved: 0, last: null },
      lastRouting: null,
    }),
  tokens: null,
  contextWindowSize: null,
  queuedMessages: [],
  collabSessions: [],
  mcpInvocations: [],
  lastRouting: null,
  configReloadedAt: null,
  planFailover: null,
  compactions: { count: 0, removedMessages: 0, tokensSaved: 0, last: null },
  toasts: [],

  subscribe: () => {
    if (get().subscribed) return () => {};
    set({ subscribed: true });
    const unsubscribe = subscribeAgentEvent((event: ReflectEvent) => {
      const patch = reduceEvent(get(), event);
      if (Object.keys(patch).length > 0) set(patch);
      // C：turn 收尾后自动排空跟进队列（reduce 完成后再取最新 state 判断）。
      if (event.msg.type === 'turn_complete' || event.msg.type === 'turn_aborted') {
        void get().drainQueue();
      }
      // coding plan 额度耗尽 → 自动切换默认供应商（内部有开关 + 防抖）。
      if (event.msg.type === 'quota_exhausted' || event.msg.type === 'error') {
        const signal = extractExhaustionSignal(event.msg, get().session?.provider ?? null);
        if (signal) void planFailover.handle(signal);
      }
      // plan 最大上下文 ≥80% → 自动压缩（内部有开关/节流/触发闩）。
      if (event.msg.type === 'token_count') {
        void autoCompact(event.msg.input_tokens ?? 0);
      }
    });
    return () => {
      unsubscribe();
      set({ subscribed: false });
    };
  },

  submit: async (text, workspace) => {
    const id = uuid();
    const submission: ReflectSubmission = {
      id,
      op: { type: 'user_input', items: [{ type: 'text', text }] },
      ...(workspace ? { workspace } : {}),
    };
    set((state) => ({
      turns: [
        ...state.turns,
        { id, items: [{ kind: 'user_text', text }], status: 'streaming' },
      ],
    }));
    try {
      await reflect_submit(submission);
    } catch (error) {
      // 提交失败：乐观 turn 落为 aborted 并记录错误。否则它永远停在
      // streaming → selectIsTurnRunning 恒真 → 队列永不排空,整个会话假死。
      set((state) => ({
        turns: markTurn(state.turns, id, 'aborted'),
        lastError: error instanceof Error ? error.message : String(error),
      }));
      throw error;
    }
  },

  submitItems: async (items, workspace) => {
    const id = uuid();
    const submission: ReflectSubmission = {
      id,
      op: { type: 'user_input', items },
      ...(workspace ? { workspace } : {}),
    };
    const firstText = items.find(
      (item): item is { type: 'text'; text: string } => item.type === 'text',
    );
    set((state) => ({
      turns: [
        ...state.turns,
        {
          id,
          items: firstText
            ? [{ kind: 'user_text' as const, text: firstText.text }]
            : [
                {
                  kind: 'user_text' as const,
                  text: `(${items.length} attachment${items.length === 1 ? '' : 's'})`,
                },
              ],
          status: 'streaming' as const,
        },
      ],
    }));
    try {
      await reflect_submit(submission);
    } catch (error) {
      // 同 submit:失败回滚乐观 turn,不卡死队列。
      set((state) => ({
        turns: markTurn(state.turns, id, 'aborted'),
        lastError: error instanceof Error ? error.message : String(error),
      }));
      throw error;
    }
  },

  interrupt: async () => {
    await reflect_interrupt();
  },

  enqueueMessage: (items, workspace) => {
    const firstText = items.find(
      (item): item is { type: 'text'; text: string } => item.type === 'text',
    );
    set((state) => ({
      queuedMessages: [
        ...state.queuedMessages,
        {
          id: uuid(),
          items,
          text: firstText?.text ?? `(${items.length} attachments)`,
          ...(workspace ? { workspace } : {}),
          createdAt: Date.now(),
        },
      ],
    }));
  },

  removeQueued: (id) => {
    set((state) => ({ queuedMessages: state.queuedMessages.filter((m) => m.id !== id) }));
  },

  updateQueued: (id, text) => {
    set((state) => ({
      queuedMessages: state.queuedMessages.map((m) => {
        if (m.id !== id) return m;
        // 仅更新首个 text item；附件项原样保留。
        let replaced = false;
        const items = m.items.map((item) => {
          if (!replaced && item.type === 'text') {
            replaced = true;
            return { ...item, text };
          }
          return item;
        });
        if (!replaced) items.unshift({ type: 'text', text });
        return { ...m, items, text };
      }),
    }));
  },

  drainQueue: async () => {
    const state = get();
    if (state.queuedMessages.length === 0) return;
    if (selectIsTurnRunning(state)) return;
    // 审批/提问/输入/plan 等待中 → 暂停排空，处理完后由下一次事件再触发。
    if (selectHasPendingInteraction(state)) return;
    const [next, ...rest] = state.queuedMessages;
    set({ queuedMessages: rest });
    try {
      await state.submitItems(next.items, next.workspace ?? undefined);
    } catch (error) {
      // 提交失败:消息放回队首等下一次事件重试,不静默丢弃。
      // 若用户期间编辑过队列(该条已不在),仍放回最前,保证不丢内容。
      const current = get();
      const queued = current.queuedMessages.some((m) => m.id === next.id)
        ? current.queuedMessages
        : [next, ...current.queuedMessages];
      current.pushToast({
        kind: 'error',
        message: error instanceof Error ? error.message : String(error),
      });
      set({ queuedMessages: queued });
    }
  },

  compact: async () => {
    await reflect_compact();
  },
  rewind: async (toTurnId) => {
    await reflect_rewind(toTurnId);
  },
  shutdown: async () => {
    await reflect_shutdown();
  },

  approve: async (kind, id, decision) => {
    set((state) => ({
      pendingApprovals: state.pendingApprovals.filter((approval) => approval.id !== id),
    }));
    if (kind === 'tool') await reflect_tool_approval(id, decision);
    else await reflect_hook_approval(id, decision);
  },

  // plan 审批走独立的 approvePlan —— PlanApprovalChoice 三选一,语义与
  // tool/hook 的 ReviewDecision 不同,故单独方法。
  approvePlan: async (id, choice) => {
    set((state) => ({
      pendingApprovals: state.pendingApprovals.filter((approval) => approval.id !== id),
    }));
    await reflect_plan_approval(id, choice);
  },

  enterPlanMode: async (task) => {
    await reflect_enter_plan_mode(task);
  },
  exitPlanMode: async () => {
    await reflect_exit_plan_mode();
  },
  setEffort: async (level) => {
    await reflect_set_effort(level);
  },
  setPermissionMode: async (mode) => {
    await reflect_set_permission_mode(mode);
  },
  cyclePermissionMode: async () => {
    await reflect_cycle_permission_mode();
  },
  syncPermissionMode: (mode) => {
    set({ permissionMode: mode });
  },

  answerQuestion: async (id, answers) => {
    set((state) => ({
      pendingQuestions: state.pendingQuestions.filter((question) => question.id !== id),
    }));
    await reflect_ask_user_question_response(id, answers);
  },

  answerInput: async (id, text) => {
    set((state) => ({
      pendingAskUser: state.pendingAskUser.filter((question) => question.id !== id),
    }));
    await reflect_ask_user_input_response(id, text);
  },

  clearError: () => set({ lastError: null }),
  ...createToastActions(set),

  reset: () => {
    planFailover.reset();
    set({
      turns: [],
      session: null,
      loadedSessionId: null,
      permissionMode: 'auto',
      pendingApprovals: [],
      pendingQuestions: [],
      pendingAskUser: [],
      pendingPlan: null,
      mcpServers: [],
      lspServers: [],
      lastError: null,
      tokens: null,
      contextWindowSize: null,
      queuedMessages: [],
      collabSessions: [],
      mcpInvocations: [],
      lastRouting: null,
      configReloadedAt: null,
      planFailover: null,
      compactions: { count: 0, removedMessages: 0, tokensSaved: 0, last: null },
      toasts: [],
    });
  },
  };
});
