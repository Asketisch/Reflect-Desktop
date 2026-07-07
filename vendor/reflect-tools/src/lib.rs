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
//! `reflect-tools` — Tool trait, builtin tools, and the partitioned
//! `ToolExecutionQueue` that drives the M2+ agent loop.

pub mod approval;
pub mod bm25;
pub mod builtins;
pub mod hashline;
pub mod plan_approval;
pub mod queue;
pub mod registry;
pub mod sandbox;
pub mod sanitize;
pub mod checkpoint;
pub mod spec;
pub mod tool;
pub mod vector_search;
pub mod worktree;

pub use approval::{
    ApprovalGate, ApprovalWaiters, AskUserInputWaiters, AskUserQuestionWaiters, complete_approval,
    complete_ask_user_input, complete_ask_user_question,
};
pub use bm25::{Bm25Hit, rank_lines, tokenize};
pub use builtins::bash::{BashCommandClass, classify_command};
pub use hashline::{
    HashlineAnchor, apply_hashline_replace, line_hash, locate_anchor, parse_anchor,
};
pub use plan_approval::{PlanApprovalGate, PlanApprovalWaiters, complete_plan_approval};
pub use queue::{ToolCallRequest, ToolExecutionQueue, ToolResult};
pub use registry::{ToolRegistry, ToolSource};
pub use sanitize::{SanitizeConfig, SanitizeError, Sanitizer, sanitize_text};
pub use spec::ToolSpec;
pub use tool::{Tool, ToolContext};
pub use vector_search::{VectorDocument, VectorHit, VectorIndex, stub_embed};
pub use worktree::{
    SessionWorktreeState, WorktreeCoordinator, create_worktree, default_worktree_path,
    detach_worktree, git_head_ref, git_root, remove_worktree, run_git, sanitize_branch_name,
    worktree_dirty,
};
// Re-export from protocol so downstream users can use
// `reflect_tools::ToolError` / `reflect_tools::ToolOutput` without
// taking a direct dep on `reflect-protocol`.
pub use reflect_protocol::{ToolError, ToolOutput};
