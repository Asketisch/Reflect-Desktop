//! `ExitPlanMode` —— v1.x Plan mode 控制面工具。
//!
//! agent 在 Plan mode 调研完成后调用,把 plan markdown 提交给用户审批。
//! `submission_loop` 在 `ToolCallEnd` 阶段按 tool_name 派发
//! `EventMsg::PlanReady`,由 TUI 弹 plan approval modal。
//!
//! 设计要点:
//! - 无 `required_permission` override(默认 `Auto`)
//! - `is_concurrency_safe = true`:无副作用,可并发
//! - 在 `PlanModeGate` 白名单内,允许在 Plan 模式下调用(plan 调研的终点)
//! - `markdown` 字段可选:agent 可在工具里直接给完整 markdown,也可省略
//!   让 `submission_loop` 兜底从 conversation history 汇编(Phase 4 实现)

use async_trait::async_trait;
use reflect_protocol::PermissionMode;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolError};

pub struct ExitPlanModeTool;

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }

    fn description(&self) -> &str {
        "退出 Plan mode 并把调研结果(plan markdown)提交给用户审批。 \
         agent 在 Plan mode 调研完成后调用。返回一段文本确认 plan 已提交; \
         用户在 TUI modal 上 approve 后,submission_loop 把 PermissionMode \
         切回 Prompt,写工具解锁。"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {
                    "type": "string",
                    "description": "Plan 的完整 markdown 内容;若省略则 \
                                    submission_loop 兜底从 conversation history 汇编"
                }
            },
            "required": []
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    fn required_permission(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        args: Value,
    ) -> Result<reflect_protocol::ToolOutput, ToolError> {
        let markdown = args
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        // 注意:`PlanReady` event 实际由 submission_loop 在 ToolCallEnd 阶段
        // 派发,带上 args["markdown"] 和一个新的 PlanId。
        Ok(reflect_protocol::ToolOutput {
            content: vec![reflect_protocol::ContentBlock::text(match &markdown {
                Some(m) if !m.trim().is_empty() => {
                    format!("Plan ready ({len} chars)", len = m.len())
                }
                _ => {
                    "Plan ready (no markdown provided; submission_loop will assemble from history)"
                        .to_string()
                }
            })],
            is_error: false,
            metadata: serde_json::json!({"plan_markdown_len": markdown.as_ref().map(|m| m.len()).unwrap_or(0)}),
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exit_plan_mode_with_markdown_returns_confirmation() {
        let t = ExitPlanModeTool;
        let ctx = ToolContext::default();
        let markdown = "## Plan\n1. read foo.rs\n2. edit bar.rs";
        let out = t
            .execute(ctx, serde_json::json!({"markdown": markdown}))
            .await
            .unwrap();
        assert!(!out.is_error);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("Plan ready"));
            }
            _ => panic!("expected text block"),
        }
        assert_eq!(out.metadata["plan_markdown_len"], markdown.chars().count());
    }

    #[tokio::test]
    async fn exit_plan_mode_without_markdown_still_succeeds() {
        // v1.x:允许省略 markdown,由 submission_loop 兜底。
        let t = ExitPlanModeTool;
        let ctx = ToolContext::default();
        let out = t.execute(ctx, serde_json::json!({})).await.unwrap();
        assert!(!out.is_error);
        assert_eq!(out.metadata["plan_markdown_len"], 0);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("assemble from history"));
            }
            _ => panic!("expected text block"),
        }
    }

    #[test]
    fn exit_plan_mode_metadata_is_stable() {
        let t = ExitPlanModeTool;
        assert_eq!(t.name(), "ExitPlanMode");
        assert!(t.is_concurrency_safe());
        assert_eq!(t.required_permission(), PermissionMode::Auto);
        let schema = t.parameters_schema();
        // markdown 字段可选(没有出现在 required 数组里)。
        let required = schema["required"].as_array().expect("required is array");
        assert!(required.is_empty(), "markdown 字段是可选的");
    }

    #[tokio::test]
    async fn exit_plan_mode_rejects_non_string_markdown() {
        let t = ExitPlanModeTool;
        let ctx = ToolContext::default();
        // markdown 字段类型错时不应崩溃,应降级为 None(markdown 长度 0)。
        let out = t
            .execute(ctx, serde_json::json!({"markdown": 123}))
            .await
            .unwrap();
        assert!(!out.is_error);
        assert_eq!(out.metadata["plan_markdown_len"], 0);
    }

    #[tokio::test]
    async fn exit_plan_mode_treats_empty_markdown_as_missing() {
        // 空白 markdown 等同于未提供。
        let t = ExitPlanModeTool;
        let ctx = ToolContext::default();
        let out = t
            .execute(ctx, serde_json::json!({"markdown": "  \n\t  "}))
            .await
            .unwrap();
        assert!(!out.is_error);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("assemble from history"));
            }
            _ => panic!("expected text block"),
        }
    }
}
