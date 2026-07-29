//! `Compactor` — orchestrates microcompact → smart_prune → LLM-summarize.
//!
//! Mirrors reflect's escalation strategy:
//! 1. Estimate total tokens.
//! 2. If under `trigger * microcompact_trigger_ratio` → `Noop`.
//! 3. Run microcompact. If now under `trigger` → return `Microcompact`.
//! 4. Run smart_prune. If now under `target` → return `SmartPrune`.
//! 5. If `summarize_after` is true, call the LLM summarizer. If the
//!    summary fits under `target` → return `LlMSummarize`.
//! 6. Fall back to best-effort smart_prune output.
//!
//! Cycle-free: depends only on `reflect-llm` for types. The concrete
//! `Summarizer` is injected by the caller (typically `LlmSummarizer`
//! constructed in `reflect-exec`).

use std::sync::Arc;

use reflect_llm::ChatMessage;
use reflect_protocol::{ContextCompactedEvent, ContextCompactedStrategy};

use crate::microcompact::{MicrocompactConfig, microcompact};
use crate::smart_prune::{SmartPruneConfig, smart_prune};
use crate::summarizer::Summarizer;
use crate::tokens::estimate_messages;

/// Default trigger threshold. M5 v0: `max_estimated_tokens = 10000` so
/// microcompact fires frequently on long conversations; the prior M4
/// default was 160_000 (reflect). Operators can override via the
/// `REFLECT_AUTO_COMPACT_INPUT_TOKENS` env var.
pub const DEFAULT_TRIGGER_TOKENS: u32 = 10_000;

/// Selected compaction strategy for a single `compact()` call. Also
/// stored in `ContextCompactedEvent::strategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// No strategy was needed.
    Noop,
    /// Local heuristic, no LLM.
    Microcompact,
    /// Threshold-triggered heuristic, no LLM.
    SmartPrune,
    /// LLM-generated summary.
    LlMSummarize,
}

impl From<CompactionStrategy> for ContextCompactedStrategy {
    fn from(s: CompactionStrategy) -> Self {
        match s {
            CompactionStrategy::Noop => ContextCompactedStrategy::Noop,
            CompactionStrategy::Microcompact => ContextCompactedStrategy::Microcompact,
            CompactionStrategy::SmartPrune => ContextCompactedStrategy::SmartPrune,
            CompactionStrategy::LlMSummarize => ContextCompactedStrategy::LlMSummarize,
        }
    }
}

/// Tunable parameters for the compactor.
#[derive(Debug, Clone)]
pub struct CompactorConfig {
    /// Total tokens above which compaction kicks in.
    pub trigger_tokens: u32,
    /// Below `trigger * ratio` the input is a no-op.
    pub microcompact_ratio: f32,
    /// `keep_recent` for microcompact.
    pub keep_recent_microcompact: usize,
    /// `keep_recent` for smart_prune.
    pub keep_recent_smart_prune: usize,
    /// `target_tokens` for smart_prune (default = trigger * 0.75).
    pub target_ratio: f32,
    /// Whether to call the LLM summarizer when smart_prune is still
    /// over budget.
    pub summarize_after: bool,
}

impl Default for CompactorConfig {
    fn default() -> Self {
        Self {
            trigger_tokens: DEFAULT_TRIGGER_TOKENS,
            microcompact_ratio: 0.7,
            keep_recent_microcompact: 30,
            keep_recent_smart_prune: 40,
            target_ratio: 0.75,
            summarize_after: true,
        }
    }
}

impl CompactorConfig {
    /// Compute the smart_prune target from `trigger * target_ratio`.
    pub fn target_tokens(&self) -> u32 {
        ((self.trigger_tokens as f32) * self.target_ratio) as u32
    }
}

/// The orchestrator. Holds a config and an injected summarizer.
pub struct Compactor {
    cfg: CompactorConfig,
    summarizer: Arc<dyn Summarizer>,
}

impl std::fmt::Debug for Compactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compactor")
            .field("cfg", &self.cfg)
            .field("summarizer", &"<dyn Summarizer>")
            .finish()
    }
}

impl Compactor {
    /// New compactor with the given config and summarizer.
    pub fn new(cfg: CompactorConfig, summarizer: Arc<dyn Summarizer>) -> Self {
        Self { cfg, summarizer }
    }

    /// Current config.
    pub fn config(&self) -> &CompactorConfig {
        &self.cfg
    }

    /// Run compaction. Returns the new message list and a
    /// `ContextCompactedEvent` reflecting the chosen strategy.
    ///
    /// The event has `removed_messages`, `before_tokens`, `after_tokens`
    /// filled in. `strategy` is one of `Noop` / `Microcompact` /
    /// `SmartPrune` / `LlMSummarize`.
    ///
    /// `prev_summary` is the last LLM-generated summary (if any) — when
    /// `Some`, the LLM-summarize step calls `summarize_recent` instead of
    /// `summarize_full`, producing an incremental update rather than a
    /// from-scratch re-summarization.
    pub async fn compact(
        &self,
        messages: Vec<ChatMessage>,
    ) -> (Vec<ChatMessage>, ContextCompactedEvent) {
        self.compact_with_prior_and_tokens(messages, None, None)
            .await
    }

    /// Variant of [`compact`] that accepts an optional prior summary.
    /// When `prev_summary.is_some()` and the LLM-summarize step fires,
    /// `summarize_recent(prev, msgs)` is called instead of
    /// `summarize_full(msgs)`. The LLM-reported input token signal is
    /// not provided (`None`); use [`compact_with_prior_and_tokens`]
    /// directly when callers have an LLM-reported total to pass.
    pub async fn compact_with_prior(
        &self,
        messages: Vec<ChatMessage>,
        prev_summary: Option<&str>,
    ) -> (Vec<ChatMessage>, ContextCompactedEvent) {
        self.compact_with_prior_and_tokens(messages, prev_summary, None)
            .await
    }

    /// Full entry point. `llm_reported_input_tokens`, when `Some(n)`,
    /// supplies the input-token count from the LLM's last `Usage` event
    /// (Anthropic's `input_tokens` already includes the `cache_creation`
    /// subsegment, so this is the on-the-wire billed input). When `None`
    /// or smaller than the local estimate, the local `estimate_messages`
    /// heuristic is used as the conservative fallback. The trigger
    /// threshold uses `max(estimate, llm_reported)` so the compactor
    /// always fires when **either** signal crosses the threshold — this
    /// is the safe default because:
    /// - if the LLM reported 0 abnormally, the local estimate still catches
    ///   oversized prompts;
    /// - if the local estimate under-counts (e.g. system tools + skills
    ///   catalog not yet counted), the LLM-reported total is authoritative.
    pub async fn compact_with_prior_and_tokens(
        &self,
        messages: Vec<ChatMessage>,
        prev_summary: Option<&str>,
        llm_reported_input_tokens: Option<u32>,
    ) -> (Vec<ChatMessage>, ContextCompactedEvent) {
        let local_estimate = estimate_messages(&messages);
        let before_tokens = match llm_reported_input_tokens {
            Some(n) => n.max(local_estimate),
            None => local_estimate,
        };
        let micro_threshold =
            ((self.cfg.trigger_tokens as f32) * self.cfg.microcompact_ratio) as u32;

        // Step 1: under micro-threshold → noop.
        if before_tokens < micro_threshold {
            return (
                messages,
                ContextCompactedEvent {
                    strategy: ContextCompactedStrategy::Noop,
                    removed_messages: 0,
                    before_tokens,
                    after_tokens: before_tokens,
                },
            );
        }

        // Step 2: microcompact.
        let mc_cfg = MicrocompactConfig {
            trigger_tokens: self.cfg.trigger_tokens,
            keep_recent: self.cfg.keep_recent_microcompact,
            trigger_ratio: self.cfg.microcompact_ratio,
        };
        let (after_mc, mc_report) = microcompact(messages, &mc_cfg);
        if !mc_report.was_compacted {
            // Microcompact bailed early (e.g. n <= keep_recent). Try
            // smart_prune directly.
        } else if estimate_messages(&after_mc) < self.cfg.trigger_tokens {
            let after_tokens = estimate_messages(&after_mc);
            return (
                after_mc,
                ContextCompactedEvent {
                    strategy: ContextCompactedStrategy::Microcompact,
                    removed_messages: mc_report.removed_count,
                    before_tokens,
                    after_tokens,
                },
            );
        }

        // Step 3: smart_prune.
        let sp_cfg = SmartPruneConfig {
            trigger_tokens: self.cfg.trigger_tokens,
            keep_recent: self.cfg.keep_recent_smart_prune,
            target_tokens: self.cfg.target_tokens(),
            ..Default::default()
        };
        let (after_sp, sp_report) = smart_prune(after_mc, &sp_cfg);
        let after_sp_tokens = estimate_messages(&after_sp);
        if after_sp_tokens < self.cfg.target_tokens() {
            return (
                after_sp,
                ContextCompactedEvent {
                    strategy: ContextCompactedStrategy::SmartPrune,
                    removed_messages: sp_report.removed_count,
                    before_tokens,
                    after_tokens: after_sp_tokens,
                },
            );
        }

        // Step 4: LLM summarize (if enabled). M5 v0: incremental mode
        // (`summarize_recent`) when a prior summary exists, full mode
        // otherwise.
        if self.cfg.summarize_after {
            let result = match prev_summary {
                Some(prev) => {
                    self.summarizer
                        .summarize_recent(&after_sp, Some(prev))
                        .await
                }
                None => self.summarizer.summarize_full(&after_sp).await,
            };
            match result {
                Ok(summary) => {
                    let summary_msg = vec![ChatMessage::System(format!(
                        "<summary>\n{summary}\n</summary>"
                    ))];
                    let after_tokens = estimate_messages(&summary_msg);
                    return (
                        summary_msg,
                        ContextCompactedEvent {
                            strategy: ContextCompactedStrategy::LlMSummarize,
                            removed_messages: sp_report.removed_count + 1,
                            before_tokens,
                            after_tokens,
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!(?e, "summarize failed; returning best-effort smart_prune");
                }
            }
        }

        // Fallback: return the smart_prune result.
        (
            after_sp,
            ContextCompactedEvent {
                strategy: ContextCompactedStrategy::SmartPrune,
                removed_messages: sp_report.removed_count,
                before_tokens,
                after_tokens: after_sp_tokens,
            },
        )
    }
}

/// Tag a tool result's call_id with its tool name so smart_prune and
/// microcompact know which category to apply. Format: `tool:<name>:<id>`.
pub fn tag_tool_call_id(call_id: &str, tool_name: &str) -> String {
    format!("tool:{tool_name}:{call_id}")
}

/// Same, but for tools whose results must be preserved (write/edit/todo).
/// Format: `__preserve__tool:<name>:<id>`.
pub fn tag_preserved_tool_call_id(call_id: &str, tool_name: &str) -> String {
    format!("__preserve__tool:{tool_name}:{call_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use reflect_llm::{ContentBlock, ToolResult, UserContent};

    /// Mock summarizer that returns canned output.
    struct MockSummarizer {
        canned: String,
        fail: bool,
    }

    #[async_trait]
    impl Summarizer for MockSummarizer {
        async fn summarize_full(
            &self,
            _: &[ChatMessage],
        ) -> Result<String, crate::summarizer::SummarizerError> {
            if self.fail {
                Err(crate::summarizer::SummarizerError::Cancelled)
            } else {
                Ok(self.canned.clone())
            }
        }
        async fn summarize_recent(
            &self,
            _: &[ChatMessage],
            _: Option<&str>,
        ) -> Result<String, crate::summarizer::SummarizerError> {
            self.summarize_full(&[]).await
        }
    }

    fn huge_message_list() -> Vec<ChatMessage> {
        let mut msgs = vec![ChatMessage::System("sys".into())];
        for i in 0..200 {
            let content: String = "x".repeat(1000);
            msgs.push(ChatMessage::Tool(ToolResult {
                call_id: format!("c{i}"),
                content,
                is_error: false,
            }));
        }
        msgs
    }

    #[tokio::test]
    async fn noop_when_under_micro_threshold() {
        let c = Compactor::new(
            CompactorConfig::default(),
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        let msgs = vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("hi")],
        })];
        let (_, evt) = c.compact(msgs).await;
        assert!(matches!(evt.strategy, ContextCompactedStrategy::Noop));
    }

    #[tokio::test]
    async fn microcompact_when_over_threshold() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 100,
                keep_recent_microcompact: 5,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        let msgs = huge_message_list();
        let (out, evt) = c.compact(msgs).await;
        // tokens = ~200k, threshold = 100 → definitely over.
        // With 200 tool results and keep_recent=5, microcompact will
        // replace most old tool results. If that brings it under
        // trigger_tokens=100, return Microcompact. Otherwise escalate.
        let strat = evt.strategy;
        assert!(matches!(
            strat,
            ContextCompactedStrategy::Microcompact
                | ContextCompactedStrategy::SmartPrune
                | ContextCompactedStrategy::LlMSummarize
        ));
        assert!(out.len() <= 205);
    }

    #[tokio::test]
    async fn llm_summarize_when_smart_prune_insufficient() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 100,
                keep_recent_microcompact: 5,
                keep_recent_smart_prune: 5,
                target_ratio: 0.01, // very tight target so smart_prune fails
                summarize_after: true,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: "fake summary".into(),
                fail: false,
            }),
        );
        let msgs = huge_message_list();
        let (_, evt) = c.compact(msgs).await;
        // Tight target → escalate to summarize.
        assert!(matches!(
            evt.strategy,
            ContextCompactedStrategy::LlMSummarize
        ));
    }

    #[tokio::test]
    async fn fallback_to_smart_prune_when_summarize_fails() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 100,
                keep_recent_microcompact: 5,
                keep_recent_smart_prune: 5,
                target_ratio: 0.01,
                summarize_after: true,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: true,
            }),
        );
        let msgs = huge_message_list();
        let (_, evt) = c.compact(msgs).await;
        // Summarizer failed → return best-effort smart_prune.
        assert!(matches!(evt.strategy, ContextCompactedStrategy::SmartPrune));
    }

    #[test]
    fn target_tokens_is_trigger_times_ratio() {
        let cfg = CompactorConfig {
            trigger_tokens: 200_000,
            target_ratio: 0.75,
            ..Default::default()
        };
        assert_eq!(cfg.target_tokens(), 150_000);
    }

    #[test]
    fn strategy_into_protocol_strategy() {
        let pairs: Vec<(CompactionStrategy, ContextCompactedStrategy)> = vec![
            (CompactionStrategy::Noop, ContextCompactedStrategy::Noop),
            (
                CompactionStrategy::Microcompact,
                ContextCompactedStrategy::Microcompact,
            ),
            (
                CompactionStrategy::SmartPrune,
                ContextCompactedStrategy::SmartPrune,
            ),
            (
                CompactionStrategy::LlMSummarize,
                ContextCompactedStrategy::LlMSummarize,
            ),
        ];
        let mut seen: Vec<ContextCompactedStrategy> = Vec::new();
        for (a, b) in pairs {
            assert_eq!(ContextCompactedStrategy::from(a), b);
            seen.push(b);
        }
        seen.sort_by_key(|s| match s {
            ContextCompactedStrategy::Noop => 0,
            ContextCompactedStrategy::Microcompact => 1,
            ContextCompactedStrategy::SmartPrune => 2,
            ContextCompactedStrategy::LlMSummarize => 3,
        });
        seen.dedup();
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn tag_tool_call_id_format() {
        assert_eq!(tag_tool_call_id("c1", "bash"), "tool:bash:c1");
        assert_eq!(
            tag_preserved_tool_call_id("c2", "write"),
            "__preserve__tool:write:c2"
        );
    }

    // ── M8 P0b: LLM-reported input tokens drive trigger ─────────────────

    /// `huge_message_list` produces ~200k tokens of local estimate. With
    /// trigger=10k, the compactor fires (whatever the LLM signal is). This
    /// is the M7 baseline — must not regress.
    #[tokio::test]
    async fn huge_list_fires_compact_without_llm_signal() {
        let c = Compactor::new(
            CompactorConfig::default(),
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        let msgs = huge_message_list();
        let (_out, evt) = c.compact_with_prior_and_tokens(msgs, None, None).await;
        assert!(!matches!(evt.strategy, ContextCompactedStrategy::Noop));
    }

    /// Local estimate = 1000 (just `huge_message_list`'s 1 system msg ≈ 1
    /// token), LLM-reported = 50_000, trigger = 10_000 → must fire because
    /// LLM-reported is authoritative.
    #[tokio::test]
    async fn llm_reported_triggers_when_local_estimate_is_small() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 10_000,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        // 1-token local estimate.
        let msgs = vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("hi")],
        })];
        let (_out, evt) = c
            .compact_with_prior_and_tokens(msgs, None, Some(50_000))
            .await;
        assert!(
            !matches!(evt.strategy, ContextCompactedStrategy::Noop),
            "LLM-reported 50k over trigger 10k should fire, got strategy {:?}",
            evt.strategy
        );
    }

    /// Local estimate = 50_000 (huge list), LLM-reported = 8_000,
    /// trigger = 10_000 → must fire because `max(estimate, llm_reported) =
    /// 50_000 > 10_000`. The LLM-reported value is *not* allowed to
    /// suppress a real local over-budget.
    #[tokio::test]
    async fn max_of_estimate_and_llm_triggers_when_estimate_is_big() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 10_000,
                keep_recent_microcompact: 5,
                keep_recent_smart_prune: 5,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        let msgs = huge_message_list();
        let (_out, evt) = c
            .compact_with_prior_and_tokens(msgs, None, Some(8_000))
            .await;
        assert!(
            !matches!(evt.strategy, ContextCompactedStrategy::Noop),
            "estimate 50k over trigger 10k must still fire even with llm=8k, got strategy {:?}",
            evt.strategy
        );
    }

    /// Local estimate > micro-threshold (trigger * 0.7), LLM-reported = 0
    /// (anomalous), trigger = 10_000 → must fire because the local
    /// estimate guards against the LLM misreporting.
    #[tokio::test]
    async fn llm_reported_zero_falls_back_to_local_estimate() {
        let c = Compactor::new(
            CompactorConfig {
                trigger_tokens: 10_000,
                keep_recent_microcompact: 5,
                keep_recent_smart_prune: 5,
                ..Default::default()
            },
            Arc::new(MockSummarizer {
                canned: String::new(),
                fail: false,
            }),
        );
        // Build a moderate message list whose local estimate lands above
        // the micro-threshold (trigger * 0.7 = 7_000). Each tool result
        // has 4000 chars ≈ 1143 tokens × 20 = 22_860 tokens.
        let mut msgs: Vec<ChatMessage> = vec![ChatMessage::System("s".repeat(100))]; // ~29 tokens
        for i in 0..20 {
            msgs.push(ChatMessage::Tool(ToolResult {
                call_id: format!("c{i}"),
                content: "x".repeat(4000),
                is_error: false,
            }));
        }
        // Sanity: local estimate should be > micro-threshold (7000).
        let local = estimate_messages(&msgs);
        assert!(
            local > 7000,
            "test setup wrong: local estimate {local} should exceed micro-threshold 7000"
        );
        let (_out, evt) = c.compact_with_prior_and_tokens(msgs, None, Some(0)).await;
        assert!(
            !matches!(evt.strategy, ContextCompactedStrategy::Noop),
            "llm=0 must not suppress local-estimate over-budget, got strategy {:?}",
            evt.strategy
        );
    }
}
