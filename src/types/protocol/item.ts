/**
 * Reflect protocol —— UserInputItem 判别联合。
 *
 * 对应 `reflect-agent/crates/protocol/reflect-protocol/src/item.rs` 中的 `UserInputItem`。每个
 * 变体使用 `type` discriminator 标记,与 Rust 的
 * `#[serde(rename_all = "snake_case")]` 枚举对齐。`question_answer` 变体
 * 引用 `AskUserAnswer`(见 `./question.ts`)。
 *
 * 顶层同步警告见 `./index.ts`。
 */

import type { AskUserAnswer } from './question';

export type UserInputItem =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mime_type: string }
  | { type: 'local_image'; path: string }
  | { type: 'skill'; name: string; args?: unknown }
  | { type: 'question_answer'; request_id: string; answers: AskUserAnswer };
