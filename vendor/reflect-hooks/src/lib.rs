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
//! `reflect-hooks` — Hook trait, HookEngine, 5 events, 5 decisions, built-in hooks.
//!
//! See `docs/tools-and-hooks.md §4` for the protocol. `PermissionMode` is
//! re-exported from `reflect-protocol` (the actual definition lives there
//! to break the `reflect-tools` ↔ `reflect-hooks` dependency cycle).

pub mod abort;
pub mod builtins;
pub mod config;
pub mod decision;
pub mod engine;
pub mod event;
pub mod file_read_state;
pub mod hook;

pub use abort::HookAbortSignal;
pub use decision::{HookDecision, SystemMessage};
pub use engine::HookEngine;
pub use event::{HookContext, HookEvent, HookEventKind, StopReason};
pub use file_read_state::{DenyReason, FileReadStateTracker, ReadRecord, SharedFileReadState};
pub use hook::{Hook, HookError};

// Re-export for users who think of PermissionMode as part of the hook
// protocol (it is — it appears in `HookDecision::PermissionOverride`).
pub use reflect_protocol::PermissionMode;
