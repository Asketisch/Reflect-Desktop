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
//! reflect-prompt — layered system prompt builder and prompt caching support.
//!
//! M4 of the Reflect roadmap. Three modules:
//! - [`template`] — thin minijinja wrapper for `{{ var }}` substitution.
//! - [`caching`] — Anthropic `cache_control` injection + change detection.
//! - [`builder`] — `LayeredPrompt` composition (core / append / ephemeral)
//!   and a `PromptBuilder` that owns a `CacheBreakDetector` and assembles
//!   a `ChatRequest` from a layer set.
//!
//! v0 only does `{{ var }}` substitution; no `{% if %}` / `{% for %}` blocks
//! (matches reflect `manager.get_prompt` semantics).

pub mod builder;
pub mod caching;
pub mod template;

pub use builder::{CORE_PREFIX, EPHEMERAL_PREFIX, LayeredPrompt, MEMORY_HEADER, PromptBuilder};
pub use caching::{
    CacheBreakDetector, CacheBreakError, CacheMonitorStats, DEFAULT_PREFIX_ANCHOR_OFFSET,
    find_prefix_anchor, inject_cache_control,
};
pub use template::{PromptError, render, render_with};

/// Default maximum character length for a tool spec line in the ephemeral
/// reminder (matched against reflect's `max_result_chars`).
pub const MAX_TOOL_LINE_CHARS: usize = 200;
