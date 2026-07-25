/**
 * Reflect protocol — shared core enums (string unions) used by every
 * domain module. This file only contains *type-level* enums (no payload
 * structs, no discriminated unions) so it can be safely imported by any
 * other module without creating circular references.
 *
 * See `./index.ts` for the full barrel re-export and the top-level
 * vendor-sync warning comment.
 */

export type PermissionMode =
  | 'auto'
  | 'prompt'
  | 'deny'
  | 'plan'
  | 'accept_edits'
  | 'bubble'
  | 'bypass';

export type ApprovalPolicy = 'auto' | 'prompt' | 'deny';
export type SandboxPolicy = 'workspace_only' | 'os_sandbox' | 'full_access';
export type RiskLevel = 'low' | 'medium' | 'high';
export type ReasoningEffort = 'low' | 'medium' | 'high';

/** ReviewDecision is snake_case-tagged. Deny carries `{ reason: string }`. */
export type ReviewDecision = 'approve' | { deny: { reason: string } } | 'approve_for_session';
