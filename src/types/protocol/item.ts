/**
 * Reflect protocol — UserInputItem discriminated union.
 *
 * Mirrors `vendor/reflect-protocol/src/item.rs` `UserInputItem`. Each
 * variant is tagged with a `type` discriminator matching the Rust
 * `#[serde(rename_all = "snake_case")]` enum. The `question_answer`
 * variant references `AskUserAnswer` (see `./question.ts`).
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

import type { AskUserAnswer } from './question';

export type UserInputItem =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mime_type: string }
  | { type: 'local_image'; path: string }
  | { type: 'skill'; name: string; args?: unknown }
  | { type: 'question_answer'; request_id: string; answers: AskUserAnswer };
