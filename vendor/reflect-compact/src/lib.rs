#![allow(clippy::derivable_impls)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::io_other_error)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::manual_div_ceil)]
//! reflect-compact — context compaction strategies.
//!
//! M4 of the Reflect roadmap. Port of reflect's `graph.py:330-609`
//! (microcompact + smart_prune) and `summarizer.py` (LLM-summarize).
//!
//! Three strategies, plus a noop:
//!
//! - [`microcompact`] — local heuristic, no LLM. Pins `System` + first
//!   `User`; replaces old tool results with placeholders; strips thinking.
//! - [`smart_prune`] — local heuristic, no LLM. Per-tool truncation rules
//!   (grep → head, bash → tail, read → head+tail, others → chars). Drops
//!   oldest non-pinned until under `target_tokens`.
//! - [`summarizer::Summarizer`] + [`LlmSummarizer`] — async, LLM-backed.
//!   Generates a 9-section Chinese summary, wrapped in `<summary>` tags.
//!
//! [`strategy::Compactor`] ties them together: estimates tokens, runs
//! microcompact first, escalates to smart_prune, then LLM summarize.

pub mod microcompact;
pub mod smart_prune;
pub mod strategy;
pub mod summarizer;
pub mod tokens;
pub mod tool_pair;

pub use microcompact::{
    CompactReport, KEEP_RECENT_DEFAULT, MICROCOMPACT_TRIGGER_RATIO, MicrocompactConfig,
    PRESERVE_TOOL_NAMES, TRUNCATABLE_TOOL_NAMES, microcompact,
};
pub use smart_prune::{
    MAX_ASSISTANT_CHARS, MAX_SUBAGENT_RESULT_CHARS, MAX_TOOL_RESULT_CHARS, MAX_TOOL_RESULT_LINES,
    SmartPruneConfig, smart_prune,
};
pub use strategy::{CompactionStrategy, Compactor, CompactorConfig, DEFAULT_TRIGGER_TOKENS};
pub use summarizer::{
    LlmSummarizer, SUMMARIZE_PROMPT_FULL, SUMMARIZE_PROMPT_RECENT, SUMMARIZE_TIMEOUT, Summarizer,
    SummarizerError,
};
pub use tokens::estimate_messages;
