//! `SubAgentSpec` — declarative description of a subagent that the parent
//! can spawn via the `call_<role>` tool.

use crate::data_transfer::DataTransferConfig;

/// One named subagent. `role` determines the tool name (`call_<role>`)
/// that the parent LLM uses to invoke this agent.
#[derive(Debug, Clone)]
pub struct SubAgentSpec {
    /// Human-readable label (used in logs).
    pub name: String,
    /// Short identifier — also the suffix of the tool name. Must be
    /// snake_case and unique per parent (the registry does not check).
    pub role: String,
    /// Model spec (e.g. `"openai/gpt-4o"`); defaults to the parent's model
    /// when `None`.
    pub model: Option<String>,
    /// System prompt prepended to the child's conversation.
    pub system_prompt: String,
    /// Subset of the parent's tool names the child can invoke. Empty =
    /// no tools (text-only child).
    pub allowed_tools: Vec<String>,
    /// How the parent's context is passed in and the child's final answer
    /// is extracted back.
    pub data_transfer: DataTransferConfig,
}

impl SubAgentSpec {
    /// `call_<role>` — the name exposed to the parent LLM.
    pub fn tool_name(&self) -> String {
        format!("call_{}", self.role)
    }

    /// Validate the spec. Returns `Err` if `role` is empty or contains
    /// characters that would confuse the JSON-Schema parameter list.
    pub fn validate(&self) -> Result<(), String> {
        if self.role.is_empty() {
            return Err("role cannot be empty".into());
        }
        if !self
            .role
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(format!(
                "role '{}' must be lowercase alphanumeric + _-",
                self.role
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_name_from_role() {
        let s = SubAgentSpec {
            name: "Explorer".into(),
            role: "explorer".into(),
            model: None,
            system_prompt: "x".into(),
            allowed_tools: vec![],
            data_transfer: DataTransferConfig::default(),
        };
        assert_eq!(s.tool_name(), "call_explorer");
    }

    #[test]
    fn validate_accepts_lowercase_role() {
        let s = SubAgentSpec {
            name: "x".into(),
            role: "explorer-v2".into(),
            model: None,
            system_prompt: "x".into(),
            allowed_tools: vec![],
            data_transfer: DataTransferConfig::default(),
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn validate_rejects_uppercase() {
        let s = SubAgentSpec {
            name: "x".into(),
            role: "Explorer".into(),
            model: None,
            system_prompt: "x".into(),
            allowed_tools: vec![],
            data_transfer: DataTransferConfig::default(),
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_role() {
        let s = SubAgentSpec {
            name: "x".into(),
            role: "".into(),
            model: None,
            system_prompt: "x".into(),
            allowed_tools: vec![],
            data_transfer: DataTransferConfig::default(),
        };
        assert!(s.validate().is_err());
    }
}
