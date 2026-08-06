import type { UserInputItem } from '@/types/protocol';
import type { RiskLevel } from '@/types/protocol/enums';
import type { PlanApprovalChoice, ReflectRolloutRecord, ReviewDecision } from '@/utils/types';

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
  /** tool / hook 审批;plan 审批走独立的 PendingPlan + PlanReadyModal(ApprovePlan)。 */
  kind: 'tool' | 'hook';
  toolName?: string;
  argsSummary?: string;
  /** Risk level for permission-bubble approvals (omitted on non-bubble flows). */
  risk?: RiskLevel;
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
  /** cache_creation_input_tokens(M8)—— input 的子集,不加进 total。 */
  cacheWrite: number;
  total: number;
  cost: number | null;
  /** 命中的 provider(v1.0 多 Provider 路由),未上报时为 null。 */
  provider?: string | null;
  /** 命中的 credential label,未上报时为 null。 */
  credentialLabel?: string | null;
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
  /** 当前会话的 context window 大小(来自 session_configured),未知时为 null。 */
  contextWindowSize: number | null;
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
    kind: 'tool' | 'hook',
    id: string,
    decision: ReviewDecision,
  ) => Promise<void>;
  /** Plan 审批 —— 三选一 PlanApprovalChoice(语义与 tool/hook 的 ReviewDecision 不同)。 */
  approvePlan: (id: string, choice: PlanApprovalChoice) => Promise<void>;
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
