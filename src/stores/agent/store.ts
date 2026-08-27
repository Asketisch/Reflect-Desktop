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
  reflect_hook_approval,
  reflect_interrupt,
  reflect_plan_approval,
  reflect_rewind,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_shutdown,
  reflect_submit,
  reflect_tool_approval,
} from '@/utils/commands';
import { turnsFromRollout } from '../replay';
import { reduceEvent } from './reducer';
import { createToastActions, uuid } from './toast';
import type { AgentState } from './types';

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
  loadedSessionId: null,
  hydrateSession: (id, records) =>
    set({ turns: turnsFromRollout(records), loadedSessionId: id, lastError: null }),
  clearSession: () =>
    set({
      turns: [],
      loadedSessionId: null,
      pendingApprovals: [],
      pendingQuestions: [],
      pendingAskUser: [],
      pendingPlan: null,
    }),
  tokens: null,
  contextWindowSize: null,
  collabSessions: [],
  mcpInvocations: [],
  lastRouting: null,
  configReloadedAt: null,
  toasts: [],

  subscribe: () => {
    if (get().subscribed) return () => {};
    set({ subscribed: true });
    const unsubscribe = subscribeAgentEvent((event: ReflectEvent) => {
      const patch = reduceEvent(get(), event);
      if (Object.keys(patch).length > 0) set(patch);
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
    await reflect_submit(submission);
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
    await reflect_submit(submission);
  },

  interrupt: async () => {
    await reflect_interrupt();
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

  reset: () =>
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
      collabSessions: [],
      mcpInvocations: [],
      lastRouting: null,
      configReloadedAt: null,
      toasts: [],
    }),
}));
