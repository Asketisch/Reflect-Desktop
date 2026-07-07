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
//! reflect-core — AgentThread, submission_loop, and (in M2+) the 4-node StateGraph.
//!
//! M1 ships the minimum needed to drive a single-turn LLM call:
//! - `AgentConfig` — the runtime config
//! - `AgentThread` — owns the submission channel and the global event fan-out task
//! - `TurnHandle` — per-turn event receiver (M2+ will keep this contract)
//! - `submission_loop` — processes `Op` variants and dispatches to a single-turn runner
//!
//! M2 will add: 4-node StateGraph, `ToolExecutionQueue`, multi-turn, hooks, and
//! state persistence.

pub mod agent_thread;
pub mod background_tasks;
pub mod config;
pub mod graph;
pub mod steering_queue;
pub mod submission_loop;
pub mod turn;
pub mod workspace;

pub use agent_thread::AgentThread;
pub use background_tasks::{
    BackgroundTask, BackgroundTaskQueue, BackgroundTaskStatus, spawn_background_stub,
};
pub use config::AgentConfig;
pub use steering_queue::{SteeringMessage, SteeringPriority, SteeringQueue};
pub use submission_loop::NodeContext;
pub use turn::TurnHandle;
pub use workspace::detect_project_root;
