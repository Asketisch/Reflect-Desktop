//! `ask_user` —— v1.1.0 P1 #15:LLM 向用户发起自由文本询问。
//!
//! 与 `ask_user_question`(结构化多选题)区分:只接受一条 `prompt`,
//! TUI 弹单行 input modal,用户输入通过 `Op::AskUserInputResponse` 回执。

use async_trait::async_trait;
use reflect_protocol::{ContentBlock, PermissionMode, ToolOutput};
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolError};

pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a free-text question and wait for a typed response. \
         Use when you need open-ended input (not multiple choice). \
         Blocks the turn until the user submits or cancels."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    // v1.2 review P1:bug-3:`minLength: 1` 让 LLM 在 schema
                    // 层就知道空 prompt 不会被接受,减少一两次 round-trip。
                    "type": "string",
                    "minLength": 1,
                    "description": "The question or instruction shown to the user"
                }
            },
            "required": ["prompt"],
            "additionalProperties": false
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn required_permission(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    async fn execute(&self, ctx: ToolContext, args: Value) -> Result<ToolOutput, ToolError> {
        // v1.2 review P2:bug-1:`tracing::info_span!` 包裹单次执行,字段
        // `prompt_len` 入参常量直接 record,`response_len` / `elapsed_ms`
        // / `outcome` 在结束时回填,便于事后追查 agent 究竟问什么、
        // 用户答什么、最终状态(成功 / 取消 / 超时 / 拒答)。
        let span = tracing::info_span!(
            "ask_user.execute",
            prompt_len = tracing::field::Empty,
            response_len = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
            outcome = tracing::field::Empty,
        );
        let _enter = span.enter();

        // v1.2 review P2:bug-5:运行时强制 `additionalProperties: false`。
        // schema 标注在生产侧不一定校验,这里兜底,LLM 拼出 `{"prompt":"x","extra":1}`
        // 立即得到 InvalidArgs,而不是静默吞字段。
        let obj = args.as_object().ok_or_else(|| ToolError::InvalidArgs {
            message: "ask_user: args must be a JSON object".into(),
        })?;
        if obj.len() != 1 {
            return Err(ToolError::InvalidArgs {
                message: format!(
                    "ask_user: unexpected keys (only 'prompt' allowed), got {}",
                    obj.len()
                ),
            });
        }

        // v1.2 review P1:bug-3:在 tool 层加 trim 校验,gate 层 `is_empty`
        // 检查保留作为最后兜底。LLM 发 `prompt: "   "` 在工具入口就被拒。
        let prompt = obj
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "ask_user: missing or non-string 'prompt'".into(),
            })?
            .trim();
        if prompt.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "ask_user: 'prompt' must not be empty or whitespace".into(),
            });
        }
        span.record("prompt_len", prompt.chars().count());

        let gate = ctx.approval.as_ref().ok_or_else(|| {
            ToolError::Execution(
                "ask_user: no ApprovalGate available (headless mode not supported)".into(),
            )
        })?;

        let start = std::time::Instant::now();
        // v1.2 review P1:bug-1:与 `AskUserSection::default_timeout_secs` 默认
        // 900 秒对齐(15 分钟),0 表示永不超时。`request_human_input` 路径
        // 显式传 0 保留持久化语义。
        // v1.2 review P1:bug-2:工具名传给 gate 用于 resolver 查询("ask_user")。
        let text = gate.ask_user("ask_user", prompt, &ctx.cancel, 900).await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        span.record("response_len", text.chars().count());
        span.record("elapsed_ms", elapsed_ms);
        span.record("outcome", "ok");

        Ok(ToolOutput {
            content: vec![ContentBlock::text(text.clone())],
            is_error: false,
            metadata: serde_json::json!({
                "prompt": prompt,
                "response": text,
                "elapsed_ms": elapsed_ms,
            }),
            elapsed_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_user_tool_metadata() {
        let t = AskUserTool;
        assert_eq!(t.name(), "ask_user");
        assert!(!t.is_concurrency_safe());
        assert_eq!(t.required_permission(), PermissionMode::Auto);
    }

    /// v1.2 review P1:bug-3:schema 标注 `minLength: 1`。
    #[test]
    fn ask_user_schema_rejects_empty_prompt() {
        let schema = AskUserTool.parameters_schema();
        assert_eq!(
            schema["properties"]["prompt"]["minLength"], 1,
            "schema must declare minLength=1 on prompt"
        );
        assert_eq!(schema["required"][0], "prompt");
        assert_eq!(schema["additionalProperties"], false);
    }
}
