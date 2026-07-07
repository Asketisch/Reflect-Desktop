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
//! reflect-memory — three-scope (project / local / user) persistent memory.
//!
//! M4 of the Reflect roadmap. Port of reflect `agent_memory.py`.
//!
//! Three scopes:
//!
//! - [`MemoryScope::Project`] — `{workspace}/.reflect/agent-memory/{agent_type}/MEMORY.md`.
//!   VCS-shareable; persists across sessions; user commits when ready.
//! - [`MemoryScope::User`] — `~/.reflect/agent-memory/{agent_type}/MEMORY.md`.
//!   Cross-project, machine-local.
//! - [`MemoryScope::Session`] — in-memory only. M5 will persist via the
//!   JSONL rollout; for now it lives in a `HashMap` inside the
//!   [`InMemoryStore`].
//!
//! The character cap on injected memory is
//! [`MAX_MEMORY_INJECT_CHARS`] (8000, matches reflect).

pub mod model;
pub mod scope;
pub mod store;

pub use model::{MemoryError, MemoryRecord, MemoryScope};
pub use scope::{MAX_MEMORY_INJECT_CHARS, resolve_path};
pub use store::{
    COMBINED_MEMORY_HEADER, FileMemoryStore, InMemoryStore, MemoryStore, truncate_for_injection,
};
