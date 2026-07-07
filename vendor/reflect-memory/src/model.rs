//! `MemoryScope` + `MemoryRecord` + error type.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Three memory scopes.
///
/// - `Project` — workspace-relative, VCS-shareable.
/// - `User` — home-relative, cross-project.
/// - `Session` — in-memory only; never persisted by `FileMemoryStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Project,
    User,
    Session,
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MemoryScope::Project => "project",
            MemoryScope::User => "user",
            MemoryScope::Session => "session",
        })
    }
}

/// One memory record (used by the in-memory store and the file store's
/// metadata; not the on-disk format — on disk the file content is the
/// memory body directly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub scope: MemoryScope,
    pub agent_type: String,
    /// File path or session key.
    pub path: PathBuf,
    pub content: String,
    pub updated_at: SystemTime,
}

/// Memory errors.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// I/O error (file not found, permission denied, etc.).
    #[error("memory io error: {0}")]
    Io(#[from] std::io::Error),
    /// Cannot persist `Session` scope to disk.
    #[error("session memory cannot be persisted to disk")]
    SessionNotPersisted,
    /// Scope / agent_type combination is invalid.
    #[error("invalid memory configuration: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_display_matches_serde() {
        assert_eq!(MemoryScope::Project.to_string(), "project");
        assert_eq!(MemoryScope::User.to_string(), "user");
        assert_eq!(MemoryScope::Session.to_string(), "session");
    }

    #[test]
    fn scope_serde_roundtrip() {
        for s in [
            MemoryScope::Project,
            MemoryScope::User,
            MemoryScope::Session,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let back: MemoryScope = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }
}
