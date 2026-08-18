/**
 * Reflect protocol —— Submission 信封。
 *
 * 对应 `reflect-agent/crates/protocol/reflect-protocol/src/submission.rs` 的 `Submission`。
 * Rust 端序列化为 `{ id, op, client_user_message_id?, trace? }`；
 * `op` payload 为 `./op.ts` 中的判别联合。
 *
 * 构造函数位于 `src/protocol/submissions.ts`（每个 Op 变体一个 builder）。
 * 请使用这些构造函数，不要手动构造 `ReflectSubmission` 对象。
 *
 * 顶层同步警告见 `./index.ts`。
 */

import type { ReflectSubmissionOp } from './op';
import type { W3cTraceContext } from './op';

export interface ReflectSubmission {
  id: string;
  op: ReflectSubmissionOp;
  client_user_message_id?: string;
  trace?: W3cTraceContext;
}
