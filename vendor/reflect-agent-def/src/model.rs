//! `AgentDefinition` struct + error type.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use reflect_memory::MemoryScope;

/// Sentinel `model` value meaning "inherit from caller / `AgentConfig`".
pub const MODEL_INHERIT: &str = "inherit";

/// Default agent name when none is specified.
pub const DEFAULT_AGENT_NAME: &str = "default";

/// One agent definition (parsed from a single `agent.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Required. Stable identifier.
    #[serde(default)]
    pub name: String,
    /// Required. Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Whether other agents may spawn this one.
    #[serde(default)]
    pub spawnable: bool,
    /// Mark this agent as read-only (no `write` / `edit` tools).
    #[serde(default)]
    pub readonly: bool,
    /// Whitelist of tool names the agent may use. Empty = all builtins.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Explicit denylist of tool names.
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
    /// `Some("inherit")` → use caller-supplied model. `Some(other)` →
    /// override. `None` → use caller-supplied.
    #[serde(default)]
    pub model: Option<String>,
    /// Hard cap on iterations per turn.
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Cap on tool-result character count (per tool call).
    #[serde(default)]
    pub max_result_chars: Option<usize>,
    /// Memory scopes to load and inject.
    #[serde(default)]
    pub memory: Vec<MemoryScope>,
    /// Reserved for v1 MCP support. v0 ignores this.
    #[serde(default)]
    pub mcp_collections: Vec<String>,
    /// Markdown body (the system prompt). Comes from the part of the
    /// file after the closing `---` of the frontmatter.
    #[serde(default)]
    pub system_prompt: String,
}

impl Default for AgentDefinition {
    fn default() -> Self {
        Self {
            name: DEFAULT_AGENT_NAME.to_string(),
            description: String::new(),
            spawnable: false,
            readonly: false,
            tools: Vec::new(),
            disallowed_tools: Vec::new(),
            model: None,
            max_turns: None,
            max_result_chars: None,
            memory: Vec::new(),
            mcp_collections: Vec::new(),
            system_prompt: String::new(),
        }
    }
}

/// Agent definition errors.
#[derive(Debug, Error)]
pub enum AgentDefError {
    /// I/O error.
    #[error("agent def io error: {0}")]
    Io(#[from] std::io::Error),
    /// YAML parse error.
    #[error("agent def yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// Required field missing.
    #[error("agent def missing required field: {0}")]
    MissingField(&'static str),
    /// Frontmatter not found.
    #[error("agent def missing frontmatter (expected `---\\n...\\n---\\n`)")]
    NoFrontmatter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_documented_defaults() {
        let d = AgentDefinition::default();
        assert_eq!(d.name, DEFAULT_AGENT_NAME);
        assert!(!d.spawnable);
        assert!(!d.readonly);
        assert!(d.tools.is_empty());
        assert!(d.model.is_none());
        assert!(d.system_prompt.is_empty());
    }
}
