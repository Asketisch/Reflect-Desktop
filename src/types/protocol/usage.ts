/**
 * Reflect protocol — token usage accounting struct.
 *
 * Mirrors `vendor/reflect-protocol/src/usage.rs`. Shared by
 * `TurnCompletePayload`, `CollabMessagePayload`, and extended by
 * `TokenCountPayload` (see `./event.ts`).
 *
 * See `./index.ts` for the top-level vendor-sync warning.
 */

export interface TokenUsagePayload {
  input_tokens: number;
  output_tokens: number;
  cached_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd?: number;
  provider?: string;
  credential_label?: string;
}
