/**
 * Reflect protocol — AskUserInput payload (free-text prompt).
 *
 * Mirrors `vendor/reflect-protocol/src/event_msg.rs` `AskUserInputMsg`.
 * Distinct from `ask_user_question` (structured multi-select). The Rust
 * side uses `request_id` so the frontend can correlate the eventual
 * `Op::AskUserInputResponse` with the right prompt.
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

export interface AskUserInputPayload {
  request_id: string;
  prompt: string;
}
