//! `ToolSpec` — the model-facing description of a tool.

use serde::{Deserialize, Serialize};

use reflect_protocol::PermissionMode;

/// OpenAI/Anthropic-compatible description of a callable tool.
///
/// M1: only `Function` (the standard JSON-Schema tools shape).
/// M2+: `ProviderBuiltin` and `Mcp` for provider-native and MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSpec {
    Function {
        name: String,
        description: String,
        parameters: serde_json::Value,
        /// Permission the tool requires (M3+; default `Auto`).
        #[serde(default)]
        required_permission: PermissionMode,
    },
}

impl ToolSpec {
    /// Tool name (only valid for `Function`; will panic for other variants).
    pub fn name(&self) -> &str {
        match self {
            ToolSpec::Function { name, .. } => name,
        }
    }

    /// Effective permission required.
    pub fn required_permission(&self) -> PermissionMode {
        match self {
            ToolSpec::Function {
                required_permission,
                ..
            } => *required_permission,
        }
    }
}
