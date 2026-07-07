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
//! `reflect-subagent` — Tool-per-Agent subagent factory and session forking.
//!
//! Each subagent is exposed to the parent LLM as a tool named `call_<role>`.
//! When invoked, the factory spawns a fresh `AgentThread` (depth + 1), runs a
//! single user-input turn, and returns the extracted result. Nesting is
//! capped at `MAX_DEPTH = 3`; deeper spawns return `SubAgentError::MaxDepthExceeded`.
//!
//! `Session::fork(branch_name)` is a thin helper that allocates a child
//! `ThreadId` and emits a `RolloutRecord::Fork` into the parent's recorder so
//! the relationship between parent and child sessions is queryable on
//! resume.

pub mod data_transfer;
pub mod error;
pub mod factory;
pub mod session_fork;
pub mod spec;
pub mod tools;
pub mod worker_registry;

pub use data_transfer::{DataTransferConfig, ResultExtractor};
pub use error::SubAgentError;
pub use factory::SubAgentFactory;
pub use session_fork::ForkedSession;
pub use spec::SubAgentSpec;
pub use tools::CallSubAgentTool;
pub use worker_registry::{INTERNAL_WORKER_TOOLS, build_worker_tool_registry};

/// Maximum subagent nesting depth (parent + 3 descendants).
pub const MAX_DEPTH: u8 = 3;
