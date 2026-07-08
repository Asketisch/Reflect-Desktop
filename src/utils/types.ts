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

export type ReviewDecision = 'approve' | 'deny' | 'abort';