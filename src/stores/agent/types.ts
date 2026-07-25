import type { UserInputItem } from '@/types/protocol';
import type { ReflectRolloutRecord, ReviewDecision } from '@/utils/types';

export type TurnItem =
  | { kind: 'user_text'; text: string }
  | { kind: 'assistant_text'; text: string; streaming?: boolean }
  | { kind: 'thinking'; text: string }
  | {
      kind: 'tool_call';
      toolName: string;
      argsSummary: string;
      callId: string;
      status: 'running' | 'done' | 'error';
    }
  | { kind: 'tool_output'; callId: string; text: string; isError: boolean }
  | { kind: 'error'; text: string }
  | { kind: 'compacted'; summary: string };

export type TurnStatus = 'streaming' | 'done' | 'aborted';

export interface Turn {
  id: string;
  items: TurnItem[];
  status: TurnStatus;
}

export interface PendingApproval {
  id: string;
  kind: 'tool' | 'hook' | 'plan';
  toolName?: string;
  argsSummary?: string;
  turnId: string;
}

export interface PendingQuestion {
  id: string;
  payload: unknown;
  turnId: string;
}

export interface PendingAskUser {
  id: string;
  payload: unknown;
  turnId: string;
}

export interface PendingPlan {
  id: string;
  payload: unknown;
  turnId: string;
}

export interface TokenSnapshot {
  input: number;
  output: number;
  cached: number;
  total: number;
  cost: number | null;
}

export interface McpInvocation {
  server: string;
  tool: string;
  callId: string;
  at: number;
}

export interface CollabSession {
  id: string;
  participants: string[];
  mode: string;
  startedAt: number;
  status: 'running' | 'done';
  outcome?: string;
  rounds?: number;
  messages: CollabMessage[];
}

export interface CollabMessage {
  from: string;
  kind: string;
  content: string;
  round: number;
  at: number;
}

export interface RoutingSnapshot {
  kind: 'switched' | 'failed_over' | 'cooldown_started' | 'cooldown_cleared';
  role: string;
  from?: string;
  to?: string;
  reason: string;
}

export type ToastKind = 'info' | 'warn' | 'error' | 'success';

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  ttlMs: number;
  createdAt: number;
}

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

export interface AgentSession {
  model: string;
  provider: string;
}

export interface AgentState {
  turns: Turn[];
  session: AgentSession | null;
  permissionMode: string;
  pendingApprovals: PendingApproval[];
  pendingQuestions: PendingQuestion[];
  pendingAskUser: PendingAskUser[];
  pendingPlan: PendingPlan | null;
  mcpServers: McpServerEntry[];
  lspServers: LspServerEntry[];
  lastError: string | null;
  subscribed: boolean;
  loadedSessionId?: string | null;
  hydrateSession?: (id: string, records: ReflectRolloutRecord[]) => void;
  clearSession?: () => void;
  tokens: TokenSnapshot | null;
  collabSessions: CollabSession[];
  mcpInvocations: McpInvocation[];
  lastRouting: RoutingSnapshot | null;
  configReloadedAt: number | null;
  toasts: Toast[];

  subscribe: () => () => void;
  submit: (text: string) => Promise<void>;
  submitItems: (items: UserInputItem[]) => Promise<void>;
  interrupt: () => Promise<void>;
  compact: () => Promise<void>;
  rewind: (toTurnId?: string) => Promise<void>;
  shutdown: () => Promise<void>;
  approve: (
    kind: 'tool' | 'hook' | 'plan',
    id: string,
    decision: ReviewDecision,
  ) => Promise<void>;
  enterPlanMode: (task: string) => Promise<void>;
  exitPlanMode: () => Promise<void>;
  setEffort: (level: 'low' | 'medium' | 'high') => Promise<void>;
  setPermissionMode: (mode: string) => Promise<void>;
  cyclePermissionMode: () => Promise<void>;
  answerQuestion: (id: string, answers: unknown) => Promise<void>;
  answerInput: (id: string, text: string) => Promise<void>;
  clearError: () => void;
  pushToast: (toast: { kind: ToastKind; message: string; ttlMs?: number }) => string;
  dismissToast: (id: string) => void;
  reset: () => void;
}
