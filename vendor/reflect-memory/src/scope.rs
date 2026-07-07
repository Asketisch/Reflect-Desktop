//! Path resolution for memory scopes.
//!
//! Mirrors reflect `agent_memory.py:resolve_memory_path`.
//!
//! - `Project` → `{workspace}/.reflect/agent-memory/{agent_type}/MEMORY.md`
//! - `User`    → `{HOME}/.reflect/agent-memory/{agent_type}/MEMORY.md`
//! - `Session` → not file-backed; the [`crate::store::InMemoryStore`]
//!   handles it.

use std::path::{Path, PathBuf};

use crate::model::{MemoryError, MemoryScope};

/// Hard cap on characters injected into the system prompt from memory.
/// Mirrors reflect `_MAX_MEMORY_INJECT_CHARS`.
pub const MAX_MEMORY_INJECT_CHARS: usize = 8000;

/// Sanitize an `agent_type` for use as a directory name. Mirrors
/// reflect: replace `:` `/` `\\` with `-`.
pub fn sanitize_agent_type(agent_type: &str) -> String {
    agent_type
        .chars()
        .map(|c| match c {
            ':' | '/' | '\\' => '-',
            _ => c,
        })
        .collect()
}

/// Compute the on-disk path for a `(scope, agent_type)` pair. `Session`
/// returns an error (it has no file).
///
/// `home` defaults to `std::env::var("HOME")` (or the platform's user
/// home); tests pass an explicit value.
pub fn resolve_path(
    scope: MemoryScope,
    workspace: &Path,
    home: &Path,
    agent_type: &str,
) -> Result<PathBuf, MemoryError> {
    if agent_type.is_empty() {
        return Err(MemoryError::Invalid("agent_type is empty".into()));
    }
    let safe = sanitize_agent_type(agent_type);
    match scope {
        MemoryScope::Project => Ok(workspace
            .join(".reflect")
            .join("agent-memory")
            .join(safe)
            .join("MEMORY.md")),
        MemoryScope::User => Ok(home
            .join(".reflect")
            .join("agent-memory")
            .join(safe)
            .join("MEMORY.md")),
        MemoryScope::Session => Err(MemoryError::SessionNotPersisted),
    }
}

/// Convenience: resolve using `std::env::var("HOME")` as the home
/// directory. Errors if `$HOME` is unset.
pub fn resolve_path_default(
    scope: MemoryScope,
    workspace: &Path,
    agent_type: &str,
) -> Result<PathBuf, MemoryError> {
    let home =
        std::env::var("HOME").map_err(|_| MemoryError::Invalid("$HOME is not set".into()))?;
    resolve_path(scope, workspace, Path::new(&home), agent_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_separators() {
        assert_eq!(sanitize_agent_type("foo:bar/baz\\qux"), "foo-bar-baz-qux");
        assert_eq!(sanitize_agent_type("plain"), "plain");
        assert_eq!(sanitize_agent_type(""), "");
    }

    #[test]
    fn project_path_is_workspace_relative() {
        let p = resolve_path(
            MemoryScope::Project,
            Path::new("/work"),
            Path::new("/home"),
            "default",
        )
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("/work/.reflect/agent-memory/default/MEMORY.md")
        );
    }

    #[test]
    fn user_path_is_home_relative() {
        let p = resolve_path(
            MemoryScope::User,
            Path::new("/work"),
            Path::new("/home/u"),
            "default",
        )
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("/home/u/.reflect/agent-memory/default/MEMORY.md")
        );
    }

    #[test]
    fn session_path_errors() {
        let err = resolve_path(
            MemoryScope::Session,
            Path::new("/work"),
            Path::new("/home"),
            "default",
        )
        .unwrap_err();
        assert!(matches!(err, MemoryError::SessionNotPersisted));
    }

    #[test]
    fn empty_agent_type_errors() {
        let err = resolve_path(
            MemoryScope::Project,
            Path::new("/work"),
            Path::new("/home"),
            "",
        )
        .unwrap_err();
        assert!(matches!(err, MemoryError::Invalid(_)));
    }

    #[test]
    fn unsafe_agent_type_is_sanitized() {
        let p = resolve_path(
            MemoryScope::Project,
            Path::new("/work"),
            Path::new("/home"),
            "evil:agent/name",
        )
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("/work/.reflect/agent-memory/evil-agent-name/MEMORY.md")
        );
    }
}
