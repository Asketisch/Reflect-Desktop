/**
 * Compatibility entry point for the modular agent store.
 *
 * The implementation lives in `./agent`; existing imports from
 * `@/stores/agentStore` intentionally remain stable.
 */
export {
  reduceEvent,
  useAgent,
  useAgentStore,
} from './agent';
export type {
  AgentSession,
  AgentState,
  CollabMessage,
  CollabSession,
  LspServerEntry,
  McpInvocation,
  McpServerEntry,
  PendingApproval,
  PendingAskUser,
  PendingPlan,
  PendingQuestion,
  RoutingSnapshot,
  Toast,
  ToastKind,
  TokenSnapshot,
  Turn,
  TurnItem,
  TurnStatus,
} from './agent';
