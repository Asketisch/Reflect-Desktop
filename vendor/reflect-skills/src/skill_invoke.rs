//! `skill_invoke` —— 通用 SkillTool(P2 `skill-tool`)。
//!
//! 与 `LoadSkillTool` 类似,但支持 `args` 透传与 `trigger` 别名查询。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use reflect_tools::{Tool, ToolContext, ToolError, ToolOutput};

use crate::catalog::SkillsCatalog;

/// Tool 名 —— Claude Code 兼容别名 `skill`。
pub const SKILL_TOOL_NAME: &str = "skill";

/// 通用技能调用工具。
pub struct SkillTool {
    catalog: Arc<SkillsCatalog>,
}

impl std::fmt::Debug for SkillTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillTool").finish_non_exhaustive()
    }
}

impl SkillTool {
    pub fn new(catalog: Arc<SkillsCatalog>) -> Self {
        Self { catalog }
    }

    fn resolve_name(args: &serde_json::Value) -> Result<&str, ToolError> {
        args.get("name")
            .or_else(|| args.get("skill"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "skill: missing 'name' or 'skill' argument".into(),
            })
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        SKILL_TOOL_NAME
    }

    fn description(&self) -> &str {
        "Invoke a skill by name. Activates its tools and returns the skill body. \
         Accepts optional `args` object forwarded in metadata."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Skill name" },
                "skill": { "type": "string", "description": "Alias for name" },
                "args": { "type": "object", "description": "Optional invocation args" }
            }
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = Self::resolve_name(&args)?;
        let skill = self
            .catalog
            .get(name)
            .ok_or_else(|| ToolError::InvalidArgs {
                message: format!("skill not found: {name}"),
            })?;
        self.catalog.activate(name);
        let extra_args = args.get("args").cloned().unwrap_or(json!({}));
        let payload = json!({
            "name": skill.name,
            "body": skill.body,
            "activated_tools": skill.tools,
            "args": extra_args,
        });
        Ok(ToolOutput {
            content: vec![reflect_protocol::ContentBlock::Text {
                text: serde_json::to_string(&payload).unwrap_or_default(),
            }],
            is_error: false,
            metadata: json!({ "skill": name, "args": extra_args }),
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SkillMeta;
    use std::path::PathBuf;

    #[tokio::test]
    async fn skill_tool_accepts_skill_alias() {
        let cat = Arc::new(SkillsCatalog::new());
        cat.insert(SkillMeta {
            name: "x".into(),
            description: "d".into(),
            triggers: vec![],
            tools: vec!["read".into()],
            mcp_collections: vec![],
            path: PathBuf::new(),
            body: "body".into(),
            plugin_id: None,
            when_paths: vec![],
        });
        let tool = SkillTool::new(cat.clone());
        let out = tool
            .execute(
                ToolContext::default(),
                json!({"skill": "x", "args": {"k": 1}}),
            )
            .await
            .unwrap();
        assert!(cat.is_activated("x"));
        assert_eq!(out.metadata["args"]["k"], 1);
    }
}
