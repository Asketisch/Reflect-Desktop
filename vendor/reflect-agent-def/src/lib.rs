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
//! reflect-agent-def — Markdown + frontmatter agent definition parser.
//!
//! M4 of the Reflect roadmap. Port of reflect
//! `agent_definitions.py`. An agent definition is a Markdown file:
//!
//! ```markdown
//! ---
//! name: code-reviewer
//! description: Reviews code changes
//! spawnable: false
//! tools: [read, grep]
//! disallowed_tools: [bash, edit]
//! model: inherit
//! max_turns: 30
//! max_result_chars: 3000
//! memory: [project, user]
//! mcp_collections: []
//! ---
//! # Code Reviewer System Prompt
//!
//! You are a strict code reviewer...
//! ```
//!
//! The frontmatter is parsed as YAML; the body is the `system_prompt`.
//! TOML defaults can be layered underneath via [`merge::merge_with_toml`].

pub mod merge;
pub mod model;
pub mod parser;

pub use merge::merge_with_toml;
pub use model::{AgentDefError, AgentDefinition, DEFAULT_AGENT_NAME, MODEL_INHERIT};
pub use parser::{load_agents_dir, parse_agent_md, parse_agent_str};
