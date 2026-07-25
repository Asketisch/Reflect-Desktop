//! Tauri command surface, split by domain.

mod agent;
mod allowlist;
mod config;
mod error;
mod export;
mod files;
mod git;
mod hooks;
mod memory;
mod search;
mod sessions;
mod shell;
mod skills;
mod update;
mod workspaces;

pub use agent::*;
pub use allowlist::*;
pub use config::*;
pub use error::{CommandError, CommandResult};
pub use export::*;
pub use files::*;
pub use git::*;
pub use hooks::*;
pub use memory::*;
pub use search::*;
pub use sessions::*;
pub use shell::*;
pub use skills::*;
pub use update::*;
pub use workspaces::*;
