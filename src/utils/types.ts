/**
 * Shared Tauri types —— reflect-protocol 镜像 + ReflectAgent 特定类型。
 *
 * CodexMonitor 同名: `src/services/types.ts`
 *
 * Source of truth:
 * - Rust: `vendor/reflect-protocol/src/{event,event_msg,op,item,submission}.rs`
 * - TS:   `src/types/protocol.ts`（事件/Submission 判别联合）
 */

export interface ReflectSessionInfo {
  session_id: string;
  thread_id: string;
  model: string;
  provider: string;
  started_at: string;
  message_count: number;
  tool_count: number;
  token_total: number;
  cwd: string;
  display_name?: string;
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