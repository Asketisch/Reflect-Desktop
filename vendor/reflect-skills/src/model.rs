//! `SkillMeta` + errors.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One parsed SKILL.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    /// Skill identifier. Required.
    #[serde(default)]
    pub name: String,
    /// Triggering condition (human-readable). Required.
    #[serde(default)]
    pub description: String,
    /// Keywords the LLM can match against (e.g. `["review", "pr"]`).
    #[serde(default)]
    pub triggers: Vec<String>,
    /// Tools this skill activates. When non-empty, the catalog marks
    /// this skill with `[+tools: ...]` and `LoadSkillTool::execute`
    /// adds the names to the activated set.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Reserved for v1 MCP support.
    #[serde(default)]
    pub mcp_collections: Vec<String>,
    /// File path the skill was loaded from (informational).
    #[serde(skip)]
    pub path: PathBuf,
    /// Markdown body (system-prompt / workflow).
    #[serde(default)]
    pub body: String,
    /// v1.0.0-rc2: 如果该 skill 由 plugin 提供,记 plugin id 以便
    /// `remove_plugin_skills(plugin_id)` 一次性反注册。
    /// `None` = 内置或全局 skill(默认)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    /// P2:路径 glob 条件 —— workspace 相对路径匹配时自动激活。
    #[serde(default, rename = "when")]
    pub when_paths: Vec<String>,
}

/// Skill errors.
#[derive(Debug, Error)]
pub enum SkillError {
    /// I/O error.
    #[error("skill io error: {0}")]
    Io(#[from] std::io::Error),
    /// YAML parse error.
    #[error("skill yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// Required field missing.
    #[error("skill missing required field: {0}")]
    MissingField(&'static str),
    /// No frontmatter.
    #[error("skill missing frontmatter")]
    NoFrontmatter,
    /// Skill with this name not found.
    #[error("skill not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_meta_default_traits() {
        let m = SkillMeta {
            name: "x".into(),
            description: "d".into(),
            triggers: vec![],
            tools: vec![],
            mcp_collections: vec![],
            path: PathBuf::from("/tmp/x"),
            body: "body".into(),
            plugin_id: None,
            when_paths: vec![],
        };
        assert_eq!(m.name, "x");
    }
}
