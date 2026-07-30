/**
 * Shared Tauri types —— reflect-protocol 镜像 + ReflectAgent 特定类型。
 *
 * Source of truth:
 * - Rust: `vendor/reflect-protocol/src/{event,event_msg,op,item,submission}.rs`
 * - TS:   `src/types/protocol.ts`（事件/Submission 判别联合）
 */

export interface ReflectSessionInfo {
  /**
   * Mirrors `reflect_protocol::SessionInfo` as returned by
   * `reflect_list_sessions` (`vendor/reflect-protocol/src/recorder.rs::SessionInfo`).
   * 4 fields only — anything richer (provider, cwd, tool_count, …) must be
   * surfaced through a separate command, not retrofitted here.
   */
  session_id: string;
  model: string;
  /** RFC3339 timestamp (`DateTime<Utc>`). */
  started_at: string;
  message_count: number;
}

export interface ReflectRolloutRecord {
  type: string;
  [k: string]: unknown;
}

/**
 * ReviewDecision —— 对齐 Rust `reflect_protocol::ReviewDecision`。
 *
 * Rust 用 `#[serde(rename_all = "snake_case")]`,且 `Deny` 是 struct variant
 * (带 reason)。前端必须传以下形态之一,否则后端反序列化失败:
 *   - "approve"
 *   - "approve_for_session"
 *   - { deny: { reason: "..." } }
 */
export type ReviewDecision =
  | 'approve'
  | 'approve_for_session'
  | { deny: { reason: string } };

/**
 * PlanApprovalChoice —— 对齐 Rust `reflect_protocol::PlanApprovalChoice`。
 *
 * Rust 用 `#[serde(rename_all = "snake_case")]`,plan 审批三选一:
 *   - "auto_mode":切到 AcceptEdits(自动批准编辑/写入类)。
 *   - "manual_approve":切到 Prompt(逐工具审批,旧行为)。
 *   - "revise":留在 plan 模式,用户输入反馈继续 plan(等价 reject + 回到 plan 编辑)。
 */
export type PlanApprovalChoice = 'auto_mode' | 'manual_approve' | 'revise';