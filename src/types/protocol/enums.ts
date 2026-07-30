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

/**
 * PlanApprovalChoice — plan 模式审批的三选一(对应后端
 * `reflect_protocol::PlanApprovalChoice`,`#[serde(rename_all = "snake_case")]`)。
 *
 * - `auto_mode`:切到 AcceptEdits(自动批准编辑/写入类,Bash 等仍走审批)。
 * - `manual_approve`:切到 Prompt(逐工具审批,旧行为)。
 * - `revise`:留在 plan 模式,用户输入反馈继续 plan(等价 reject + 回到 plan 编辑)。
 */
export type PlanApprovalChoice = 'auto_mode' | 'manual_approve' | 'revise';
