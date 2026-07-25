/**
 * Reflect protocol — Submission envelope.
 *
 * Mirrors `vendor/reflect-protocol/src/submission.rs` `Submission`. The
 * Rust side serializes this as `{ id, op, client_user_message_id?, trace? }`;
 * the `op` payload is the discriminated union in `./op.ts`.
 *
 * Constructors live in `src/protocol/submissions.ts` (one builder per
 * Op variant). Use those instead of hand-rolling `ReflectSubmission`
 * objects inline.
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

import type { ReflectSubmissionOp } from './op';
import type { W3cTraceContext } from './op';

export interface ReflectSubmission {
  id: string;
  op: ReflectSubmissionOp;
  client_user_message_id?: string;
  trace?: W3cTraceContext;
}
