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
//! reflect-skills — SKILL.md scanning, loading and tool activation.
//!
//! M4 of the Reflect roadmap. Port of reflect `skill_loader.py`.
//!
//! A SKILL.md is a Markdown file with YAML frontmatter:
//!
//! ```markdown
//! ---
//! name: code-review
//! description: Reviews code changes
//! tools: [read, grep]
//! triggers: [review, pr]
//! ---
//! # Code Review
//!
//! You are a code reviewer...
//! ```
//!
//! At startup, [`SkillsCatalog::scan`] walks configured directories and
//! parses every `SKILL.md`. The catalog renders a textual index that
//! `pre_loop` injects as a system-reminder. The LLM calls
//! [`LoadSkillTool`] to activate a skill — this marks the skill's
//! `tools:` list as active so [`SkillsCatalog::active_tool_names`]
//! filters the LLM-visible tool schema.
//!
//! "always-on" tool names (those that should be visible to the LLM
//! without explicit skill activation) are set by the caller in
//! [`SkillsCatalog::new`]. v0 default = `{bash, read, write, edit,
//! grep, glob}`.

pub mod bundled;
pub mod catalog;
pub mod discovery;
pub mod loader;
pub mod model;
pub mod scanner;
pub mod skill_invoke;
pub mod tool;

pub use bundled::{bundled_skills, merge_bundled};
pub use catalog::{ALWAYS_ON_TOOLS, SkillsCatalog, render_catalog};
pub use discovery::{relativize, skill_matches_path, skills_for_path};
pub use loader::{parse_skill_file, parse_skill_str};
pub use model::{SkillError, SkillMeta};
pub use scanner::scan_skills_dirs;
pub use skill_invoke::{SKILL_TOOL_NAME, SkillTool};
pub use tool::LoadSkillTool;
