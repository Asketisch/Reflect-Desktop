/**
 * Reflect protocol TypeScript types — thin re-export shim.
 *
 * The actual definitions live in `./protocol/`, one file per domain:
 *   - protocol/event.ts           — Tauri `reflect_event` envelope +
 *                                   every EventMsg payload struct +
 *                                   `EventMsgByType` / `ReflectEventMsg`.
 *   - protocol/submission.ts      — `ReflectSubmission` envelope.
 *   - protocol/op.ts              — `ReflectSubmissionOp` discriminated
 *                                   union + `OpType` + `ThreadSettingsOverrides`
 *                                   + `W3cTraceContext`.
 *   - protocol/item.ts            — `UserInputItem` discriminated union.
 *   - protocol/question.ts        — `Question` / `QuestionOption` /
 *                                   `Answer` / `AskUserAnswer`.
 *   - protocol/ask_user_input.ts  — `AskUserInputPayload` (free-text prompt).
 *   - protocol/rollout.ts         — `ReflectSessionInfo` /
 *                                   `ReflectRolloutRecord`.
 *   - protocol/enums.ts           — `PermissionMode`, `ApprovalPolicy`,
 *                                   `SandboxPolicy`, `RiskLevel`,
 *                                   `ReasoningEffort`, `ReviewDecision`.
 *   - protocol/usage.ts           — `TokenUsagePayload`.
 *   - protocol/index.ts           — barrel re-export (this file's source).
 *
 * Consumers should keep importing from `@/types/protocol` (or the
 * relative `./protocol`) — every name previously exported from this
 * file is still re-exported below. Nothing else in the codebase should
 * reach into `./protocol/*.ts` directly; the per-domain split is an
 * internal organization concern.
 *
 * ## Vendor sync warning
 *
 * This directory (and therefore this shim) is **manually synchronized**
 * with `vendor/reflect-protocol` (the Rust schema for the Tauri IPC
 * envelope). Any drift between the per-domain modules and the Rust
 * types produces a wire-format bug. Update workflow:
 *   1. Edit the Rust enum/variant in `vendor/reflect-protocol/src/...`.
 *   2. Mirror it in the relevant per-domain module, preserving snake_case
 *      field names, discriminators, and the `{ deny: { reason } }` shape
 *      of `ReviewDecision::Deny`.
 *   3. If the change touches a new event type, add it to `EventMsgType`,
 *      `EventMsgByType`, and the `ReflectEventMsg` mapped type in
 *      `event.ts`.
 *   4. Run `pnpm typecheck` and `pnpm test` — must stay green.
 *
 * The `scripts/dump-ts-types.sh` (Rust `dump_schema` example) can
 * regenerate these from the JSON Schemas, but for offline dev this
 * directory is the source of truth.
 *
 * ## Invariants
 * 1. `id: string` is the per-submission correlation id (`""` for
 *    `EVENT_ID_NONE` lifecycle events).
 * 2. `msg.type` is the snake_case discriminator; TS narrowing is supported
 *    via the `EventMsgByType` map + `ReflectEvent` discriminated union.
 * 3. All `UserInputItem` variants match the Rust enum.
 * 4. `ContentBlock` matches the Rust tool-output enum.
 * 5. `ReviewDecision` is a snake_case-tagged enum; `Deny` carries a
 *    `{ reason: string }` payload (NOT PascalCase).
 */

export * from './protocol/index';
