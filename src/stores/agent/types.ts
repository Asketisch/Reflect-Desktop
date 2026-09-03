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
  | {
      kind: 'tool_output';
      callId: string;
      text: string;
      isError: boolean;
      /** edit/write 类工具产出的 unified diff（ContentBlock::Diff），无则为空。 */
      diff?: string;
      /** 工具输出的目标文件路径（metadata.path），无则为空。 */
      path?: string;
    }
  | { kind: 'error'; text: string };

/**
 * context_compacted 聚合统计 —— Inspector 概览展示（不再逐条插入对话流）。
 * 历史回放（rollout compaction record）只带 strategy / removed_count，
 * 因此 `tokensSaved` 仅累计 live 事件携带的 before − after。
 */
export interface CompactionStats {
  /** 触发次数（无变化的 noop 事件不计）。 */
  count: number;
  /** 累计移除消息数。 */
  removedMessages: number;
  /** 累计节省的估算 token（Σ before − after；回放 hydrate 的历史不含）。 */
  tokensSaved: number;
  /** 最近一次摘要（strategy + msgs + tokens）。 */
  last: string | null;
}

export type TurnStatus = 'streaming' | 'done' | 'aborted';

export interface Turn {
  id: string;
  items: TurnItem[];
  status: TurnStatus;
}

/** 跟进消息队列（C：Queue vs Steer）——运行中提交的消息先留在前端，可见可编辑。 */
export interface QueuedMessage {
  id: string;
  /** 完整 UserInputItem 列表（text + 附件），实际发送内容。 */
  items: UserInputItem[];
  /** 纯文本预览（首个 text item；用于待发送气泡展示与编辑）。 */
  text: string;
  /** 提交时注入的 workspace（入队时的现场）。 */
  workspace?: string | null;
  createdAt: number;
}

export interface PendingApproval {
  id: string;
  /** tool / hook 审批;plan 审批走独立的 PendingPlan + PlanReadyModal(ApprovePlan)。 */
  kind: 'tool' | 'hook';
  toolName?: string;
  argsSummary?: string;
  /** Permission bubble 审批的 risk level(非 bubble 流程省略)。 */
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

/** 一次 coding plan 自动切换（`stores/agent/planFailover.ts`）。 */
export interface PlanFailoverRecord {
  from: string;
  to: string;
  at: number;
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
  /** 前端侧跟进消息队列(C)。仅在 turn 运行时入队;切会话即清空。 */
  queuedMessages: QueuedMessage[];
  collabSessions: CollabSession[];
  mcpInvocations: McpInvocation[];
  lastRouting: RoutingSnapshot | null;
  configReloadedAt: number | null;
  /** coding plan 自动切换记录（额度耗尽 → 默认供应商切换），状态栏展示用。 */
  planFailover: PlanFailoverRecord | null;
  /** 上下文压缩聚合统计（context_compacted 事件 + 回放 compaction record）。 */
  compactions: CompactionStats;
  toasts: Toast[];
  /**
   * 目标模式前端投影（非后端权威状态）—— `/goal` 启动成功置 true,
   * `/goal clear` 与会话切换（hydrate/clear）置 false。协议无 goal 事件,
   * 切会话后端 rebind 也会丢 goal,所以按会话作用域处理。
   */
  goalActive: boolean;
  setGoalActive: (active: boolean) => void;

  subscribe: () => () => void;
  /**
   * v1.x：第二个可选参数 `workspace` —— 当前激活工作区(绝对路径)。
   * 注入到 `ReflectSubmission.workspace`，后端据此把该 session 归属到
   * 指定 workspace 并在 `session_meta.workspace` 字段落盘。
   * `undefined` / `null` 表示不指定(CLI / 测试场景)。
   */
  submit: (text: string, workspace?: string | null) => Promise<void>;
  submitItems: (items: UserInputItem[], workspace?: string | null) => Promise<void>;
  /** C：把一条跟进消息加入前端队列（turn 运行中不直发后端）。 */
  enqueueMessage: (items: UserInputItem[], workspace?: string | null) => void;
  /** C：移除队列中的一条待发送消息。 */
  removeQueued: (id: string) => void;
  /** C：编辑队列消息的纯文本内容（仅首个 text item）。 */
  updateQueued: (id: string, text: string) => void;
  /**
   * C：排空队列 —— 无运行中 turn 且无待处理交互（审批/提问/输入/plan）
   * 时，取出队首消息直接提交。turn_complete / turn_aborted 后自动调用。
   */
  drainQueue: () => Promise<void>;
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
  /**
   * 仅同步本地权限模式状态(不发 Op)。会话切换后由 bind 返回值调用,
   * 让徽标/切换器反映该会话恢复出的模式。
   */
  syncPermissionMode: (mode: string) => void;
  answerQuestion: (id: string, answers: unknown) => Promise<void>;
  answerInput: (id: string, text: string) => Promise<void>;
  clearError: () => void;
  pushToast: (toast: { kind: ToastKind; message: string; ttlMs?: number }) => string;
  dismissToast: (id: string) => void;
  reset: () => void;
}
