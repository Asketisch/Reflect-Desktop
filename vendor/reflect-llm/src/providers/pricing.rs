//! `pricing` — per-model cost computation.
//!
//! Holds a static table of USD-per-Mtok rates for the providers Reflect
//! supports in v0 (Anthropic Claude, OpenAI GPT-4o / o3-mini). `price(model,
//! usage)` returns the per-turn USD cost; `cumulative_cost_usd(model,
//! history)` sums across a session. Unknown models return `None` (the wire
//! event's `cost_usd` field then serializes as absent) — better to display
//! nothing than a wrong number.
//!
//! ## Cache-aware accounting (M8 P1a)
//!
//! Anthropic and OpenAI both charge cached input at a discount relative
//! to fresh input. The four segments and how they bill:
//!
//! | Segment | Anthropic factor | OpenAI factor |
//! |---------|------------------|---------------|
//! | `non_cached_input = input - cached - cache_write` | 1.0× | 1.0× |
//! | `cached` (cache_read) | 0.1× | 0.5× (auto-prompt-cache) |
//! | `cache_write` (cache_creation) | 1.25× | n/a (OpenAI auto-only) |
//! | `output` | 5.0× | 4.0× (gpt-4o) / 4.0× (o3-mini) |
//!
//! Pricing data is `const` (no network) so test fixtures stay stable.
//! Tables track Anthropic's 2026-Q2 list prices and OpenAI's 2026-Q2
//! list prices; refresh quarterly.
//!
//! ## Update cadence
//!
//! Provider list-price changes need a code patch + a M-x.y release. The
//! `cost_usd` field is informational; for billing reconciliation always
//! cross-check against the provider's dashboard.

use reflect_protocol::TokenUsage;

/// Per-Mtok rate in USD, broken out by the four billing segments. The
/// `*_factor` fields are relative to `input_price`; absolute prices are
/// derived as `segment_price = input_price * factor`. This keeps the
/// table compact and forces the relative ratios to be visible at a glance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    /// USD per 1M input tokens (non-cached segment baseline).
    pub input_price: f64,
    /// Output multiplier relative to `input_price` (Anthropic 5×, OpenAI 4×).
    pub output_factor: f64,
    /// Cache READ multiplier (Anthropic 0.1×, OpenAI auto 0.5×).
    pub cache_read_factor: f64,
    /// Cache WRITE multiplier (Anthropic 1.25×, OpenAI n/a → 0).
    pub cache_write_factor: f64,
    /// Long-form model id as exposed by the provider (e.g.
    /// `"claude-3-5-sonnet-latest"`, `"gpt-4o"`). The lookup is
    /// case-insensitive; the table stores canonical lowercase forms.
    pub canonical_id: &'static str,
}

/// Static pricing table. Lookup is O(n) over ~7 entries; cache locally
/// when callers hit the hot path.
///
/// Values are USD per 1M tokens. **Refreshed 2026-Q2**; update when
/// providers publish new list prices.
const PRICING_TABLE: &[ModelPricing] = &[
    // ── Anthropic Claude 4 family (2026-Q2) ──────────────────────────
    ModelPricing {
        canonical_id: "claude-opus-4-latest",
        input_price: 15.0,
        output_factor: 5.0,
        cache_read_factor: 0.1,
        cache_write_factor: 1.25,
    },
    ModelPricing {
        canonical_id: "claude-sonnet-4-latest",
        input_price: 3.0,
        output_factor: 5.0,
        cache_read_factor: 0.1,
        cache_write_factor: 1.25,
    },
    ModelPricing {
        canonical_id: "claude-3-5-sonnet-latest",
        input_price: 3.0,
        output_factor: 5.0,
        cache_read_factor: 0.1,
        cache_write_factor: 1.25,
    },
    ModelPricing {
        canonical_id: "claude-3-5-haiku-latest",
        input_price: 0.80,
        output_factor: 5.0,
        cache_read_factor: 0.1,
        cache_write_factor: 1.25,
    },
    // ── OpenAI GPT family (2026-Q2) ──────────────────────────────────
    ModelPricing {
        canonical_id: "gpt-4o",
        input_price: 5.0,
        output_factor: 4.0,      // 4× → $20/Mtok output
        cache_read_factor: 0.5,  // 50% off auto-prompt-cache hits
        cache_write_factor: 0.0, // OpenAI has no explicit cache write tier
    },
    ModelPricing {
        canonical_id: "gpt-4o-mini",
        input_price: 0.15,
        output_factor: 4.0,
        cache_read_factor: 0.5,
        cache_write_factor: 0.0,
    },
    ModelPricing {
        canonical_id: "o3-mini",
        input_price: 1.10,
        output_factor: 4.0,
        cache_read_factor: 0.5,
        cache_write_factor: 0.0,
    },
    // ── Ollama (local, v0.3.1) ───────────────────────────────────────
    // 本地推理不按 token 计费 —— `input_price: 0.0` 让 `price()` 返回
    // `Some(0.0)` 而不是 `None`,TUI 显示 "$0.00" 而不是 "—",与
    // "本地 = 免费" 的用户心智一致。
    //
    // 用户用 `:7b` / `:latest` / `:instruct` 等 tag 时,canonical_id 精确
    // 匹配失败 → 走 `None` 兜底(保守更好,避免给未知的 Ollama tag 算 $0)。
    ModelPricing {
        canonical_id: "llama3.2",
        input_price: 0.0,
        output_factor: 1.0,
        cache_read_factor: 0.0,
        cache_write_factor: 0.0,
    },
    ModelPricing {
        canonical_id: "qwen2.5",
        input_price: 0.0,
        output_factor: 1.0,
        cache_read_factor: 0.0,
        cache_write_factor: 0.0,
    },
    ModelPricing {
        canonical_id: "llama3.1",
        input_price: 0.0,
        output_factor: 1.0,
        cache_read_factor: 0.0,
        cache_write_factor: 0.0,
    },
    ModelPricing {
        canonical_id: "mistral-nemo",
        input_price: 0.0,
        output_factor: 1.0,
        cache_read_factor: 0.0,
        cache_write_factor: 0.0,
    },
    ModelPricing {
        canonical_id: "gemma2",
        input_price: 0.0,
        output_factor: 1.0,
        cache_read_factor: 0.0,
        cache_write_factor: 0.0,
    },
];

/// Look up pricing for `model_id` (case-insensitive). The model id is
/// matched without the `provider/` prefix; `reflect-core::graph::nodes`
/// already strips the prefix before calling.
fn lookup(model_id: &str) -> Option<&'static ModelPricing> {
    let needle = model_id.to_ascii_lowercase();
    PRICING_TABLE
        .iter()
        .find(|p| p.canonical_id.eq_ignore_ascii_case(&needle))
}

/// Whether `model_id` has a pricing entry in the static table (case-
/// insensitive, exact `canonical_id` match — same rule as [`price`]).
///
/// Used by TUI status-bar metrics to distinguish "free local model" from
/// "unpriced/unknown model" without invoking [`price`] with a throwaway
/// usage. Unknown models return `false`.
pub fn is_priced(model_id: &str) -> bool {
    lookup(model_id).is_some()
}

/// Compute the per-turn USD cost of a single `TokenUsage` for `model_id`.
///
/// Returns `None` if the model is unknown; callers should surface the
/// absence rather than silently using 0.0.
///
/// ## Formula
/// ```text
/// non_cached_input = max(0, input - cached - cache_write)
/// cost_usd = (non_cached_input * input_price
///           + cached * cache_read_factor * input_price
///           + cache_write * cache_write_factor * input_price
///           + output * output_factor * input_price) / 1_000_000
/// ```
pub fn price(model_id: &str, usage: &TokenUsage) -> Option<f64> {
    let p = lookup(model_id)?;
    let u = &usage;
    // Saturating subtraction: guard against any pathological case where
    // (cached + cache_write) > input (e.g. an upstream accounting bug).
    let non_cached = u
        .input_tokens
        .saturating_sub(u.cached_tokens)
        .saturating_sub(u.cache_write_tokens);
    let per_mtok = (non_cached as f64)
        + (u.cached_tokens as f64) * p.cache_read_factor
        + (u.cache_write_tokens as f64) * p.cache_write_factor
        + (u.output_tokens as f64) * p.output_factor;
    Some((per_mtok * p.input_price) / 1_000_000.0)
}

/// Sum the per-turn costs across a session's `TokenUsage` history.
/// Unknown-model entries in the history are dropped silently (the
/// session's cost is reported only over the entries we can price).
///
/// Returns `Some(0.0)` for an empty history (caller asked for total, we
/// return zero rather than None) and `Some(sum)` for a non-empty history
/// where at least one entry was priced. Returns `None` only if **every**
/// entry in the history is unpriced — the session never touched a known
/// model, so we can't give a meaningful number.
pub fn cumulative_cost_usd(model_id: &str, history: &[TokenUsage]) -> Option<f64> {
    if history.is_empty() {
        return Some(0.0);
    }
    let mut total = 0.0;
    let mut priced_any = false;
    for u in history {
        if let Some(c) = price(model_id, u) {
            total += c;
            priced_any = true;
        }
    }
    if priced_any { Some(total) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::TokenUsage;

    fn empty_usage() -> TokenUsage {
        TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 0,
        }
    }

    #[test]
    fn price_unknown_model_returns_none() {
        let u = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 150,
        };
        assert_eq!(price("gpt-9000-future", &u), None);
        assert_eq!(price("", &u), None);
    }

    #[test]
    fn price_zero_usage_returns_zero_for_known_model() {
        assert_eq!(price("claude-3-5-sonnet-latest", &empty_usage()), Some(0.0));
    }

    #[test]
    fn price_anthropic_sonnet_3_5_basic_no_cache() {
        // 1M input @ $3/Mtok, 1M output @ $15/Mtok → $18.00
        let u = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 2_000_000,
        };
        let c = price("claude-3-5-sonnet-latest", &u).unwrap();
        assert!((c - 18.0).abs() < 1e-9, "expected $18.00, got {c}");
    }

    #[test]
    fn price_anthropic_sonnet_3_5_with_cache_read_and_write() {
        // 1M input total: 600k fresh + 300k cache_read + 100k cache_write
        // 500k output
        // Fresh: 600_000 / 1M * 3.0 = $1.80
        // Cache read: 300_000 * 0.1 / 1M * 3.0 = $0.09
        // Cache write: 100_000 * 1.25 / 1M * 3.0 = $0.375
        // Output: 500_000 * 5 / 1M * 3.0 = $7.50
        // Total: $9.765
        let u = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cached_tokens: 300_000,
            cache_write_tokens: 100_000,
            total_tokens: 1_500_000,
        };
        let c = price("claude-3-5-sonnet-latest", &u).unwrap();
        assert!((c - 9.765).abs() < 1e-6, "expected $9.765, got {c}");
    }

    #[test]
    fn price_openai_gpt_4o_no_cache() {
        // 1M input @ $5/Mtok, 1M output @ $20/Mtok → $25.00
        let u = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 2_000_000,
        };
        let c = price("gpt-4o", &u).unwrap();
        assert!((c - 25.0).abs() < 1e-9, "expected $25.00, got {c}");
    }

    #[test]
    fn price_openai_gpt_4o_with_auto_cache() {
        // 1M input: 500k fresh + 500k cached
        // 200k output
        // Fresh: 500_000 * 5 / 1M = $2.50
        // Cache read: 500_000 * 0.5 * 5 / 1M = $1.25
        // Output: 200_000 * 4 * 5 / 1M = $4.00
        // Total: $7.75
        let u = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 200_000,
            cached_tokens: 500_000,
            cache_write_tokens: 0,
            total_tokens: 1_200_000,
        };
        let c = price("gpt-4o", &u).unwrap();
        assert!((c - 7.75).abs() < 1e-6, "expected $7.75, got {c}");
    }

    #[test]
    fn price_is_case_insensitive() {
        let u = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 1_000_000,
        };
        assert!(price("Claude-3-5-Sonnet-Latest", &u).unwrap() > 0.0);
        assert!(price("GPT-4o", &u).unwrap() > 0.0);
    }

    #[test]
    fn price_saturates_against_underreported_input() {
        // Pathological case: cached + cache_write > input. The local
        // `non_cached_input` should saturate to 0 (no negative), but
        // cache_read and cache_write segments still bill at their
        // respective multipliers — they are real billable segments
        // regardless of whether the upstream `input_tokens` accounting
        // missed them.
        //
        //   non_cached = max(0, 100 - 80 - 50) = 0
        //   cache_read = 80  * 0.1  = 8
        //   cache_write = 50 * 1.25 = 62.5
        //   output = 50 * 5 = 250
        //   per_mtok = 320.5
        //   cost = 320.5 * 3.0 / 1M = 0.0009615
        let u = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 80,
            cache_write_tokens: 50, // 80 + 50 = 130 > 100
            total_tokens: 150,
        };
        let c = price("claude-3-5-sonnet-latest", &u).unwrap();
        assert!(c >= 0.0, "cost must not go negative, got {c}");
        assert!(
            (c - 0.0009615).abs() < 1e-9,
            "expected $0.0009615 (saturated non_cached + cache segments + output), got {c}"
        );
    }

    #[test]
    fn cumulative_zero_for_empty_history() {
        assert_eq!(
            cumulative_cost_usd("claude-3-5-sonnet-latest", &[]),
            Some(0.0)
        );
    }

    #[test]
    fn cumulative_sums_all_entries_when_model_known() {
        // All entries are priced against the same known model. The
        // "drop unknown" semantics only applies to a whole history where
        // the model_id itself is unknown (see test below).
        let u1 = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 1_000_000,
        };
        let u2 = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 1_000_000,
        };
        let u_big = TokenUsage {
            input_tokens: 999_999_999,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 999_999_999,
        };
        // $3 + $3 + $2999.999997 = $3005.999997
        let total = cumulative_cost_usd("claude-3-5-sonnet-latest", &[u1, u2, u_big]).unwrap();
        assert!(
            (total - 3005.999997).abs() < 1e-3,
            "expected ~$3005.999997, got {total}"
        );
    }

    #[test]
    fn cumulative_returns_none_when_all_unknown() {
        let u = TokenUsage {
            input_tokens: 100,
            output_tokens: 0,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 100,
        };
        assert_eq!(
            cumulative_cost_usd("gpt-future-9000", &[u.clone(), u]),
            None
        );
    }

    #[test]
    fn pricing_table_lookup_finds_all_canonical_ids() {
        // Sanity: every entry in the table must be self-lookupable.
        for p in PRICING_TABLE {
            let found = lookup(p.canonical_id).unwrap();
            assert_eq!(found.canonical_id, p.canonical_id);
        }
    }

    // ── Ollama (v0.3.1) ─────────────────────────────────────────────

    #[test]
    fn ollama_models_price_zero() {
        // 本地推理 → Some(0.0) 而不是 None,TUI 显示 "$0.00" 而非 "—"。
        let u = TokenUsage {
            input_tokens: 1_000,
            output_tokens: 500,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 1_500,
        };
        for m in ["llama3.2", "qwen2.5", "llama3.1", "mistral-nemo", "gemma2"] {
            assert_eq!(price(m, &u), Some(0.0), "model {m} should price 0.0");
        }
    }

    #[test]
    fn ollama_unknown_substring_returns_none() {
        // 用户用 `llama3.2:7b` / `qwen2.5:latest` 等带 tag 的名字 → 表里
        // 没有精确匹配 → None(TUI 显示 "—",保守更好)。
        let u = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 0,
            cache_write_tokens: 0,
            total_tokens: 150,
        };
        assert_eq!(price("llama3.2:7b", &u), None);
        assert_eq!(price("qwen2.5:latest", &u), None);
        assert_eq!(price("mistral", &u), None); // 短名不在表里
    }
}
