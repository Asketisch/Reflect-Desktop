//! `ToolExecutionQueue` — runs a batch of tool calls with hook integration.
//!
//! See `docs/tools-and-hooks.md §3` for the protocol. Unsafe tools run
//! serially in submission order; concurrency-safe tools run in parallel
//! via `futures::future::join_all`.
//!
//! The queue requires a `HookEngine` (M2+); a default no-op engine can be
//! constructed via `HookEngine::new()`.
//!
//! M6 adds an optional `ApprovalGate`: when supplied to
//! `execute_all_with_gate`, tools whose `required_permission()` is `Prompt`
//! (and which the user hasn't `ApproveForSession`-ed) route through the
//! gate, emitting `EventMsg::ApprovalRequest` and waiting on a oneshot for
//! the user's `Op::ToolApproval` reply.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use reflect_hooks::{HookContext, HookEngine, HookEvent};
use reflect_protocol::{
    ContentBlock, PermissionMode, ReviewDecision, RiskLevel, ToolError, ToolOutput, TurnId,
};
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tracing::warn;

use crate::approval::ApprovalGate;
use crate::builtins::bash::{BashCommandClass, classify_command};
use crate::registry::ToolRegistry;
use crate::sanitize::{Sanitizer, sanitize_output};
use crate::tool::ToolContext;

/// A request to invoke a single tool call. Produced by the graph's
/// `model_call` node and consumed by `ToolExecutionQueue::execute_all`.
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    /// Stable id from the LLM (`ChatEvent::ToolUseStart.id`). Used to
    /// correlate results back to the model's request.
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// The result of a single tool execution, after hook dispatch and
/// serialization for the model.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub call_id: String,
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub elapsed_ms: u64,
    pub metadata: Value,
}

impl ToolResult {
    /// Wrap into a `ContentBlock::ToolResult` for the graph's
    /// `latest_content` history.
    pub fn into_content_block(self) -> ContentBlock {
        ContentBlock::ToolResult {
            call_id: self.call_id,
            output: ToolOutput {
                content: self.content,
                is_error: self.is_error,
                metadata: self.metadata,
                elapsed_ms: self.elapsed_ms,
            },
        }
    }
}

/// Default per-tool execution timeout when the `ToolContext` has none.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default max concurrent safe-tool executions.
const DEFAULT_MAX_CONCURRENCY: usize = 5;

/// Partition + dispatch a batch of tool calls, honouring the unsafe=serial
/// / safe=concurrent rule。
pub struct ToolExecutionQueue {
    registry: Arc<ToolRegistry>,
    hook_engine: Arc<HookEngine>,
    /// Per-call `ToolContext`. Each `execute_single` clones this and applies
    /// per-call overrides (`call_id`, `timeout`).
    base_ctx: ToolContext,
    semaphore: Arc<Semaphore>,
    /// v1.0.0-rc2:工具输出密钥脱敏器,在 `execute_single` 的 `Ok(Ok(_))`
    /// 分支、`tool.execute(...)` 返回之后、`PostToolUse` 派发之前应用。
    /// `Arc` 包装使克隆廉价 —— bootstrap 阶段在 `reflect-exec` 一次性构造,
    /// 后续 clone 共享同一组编译好的正则。`Sanitizer::disabled()` 等价
    /// 「no-op pass」,无须在 queue 内部做 enabled 判断。
    sanitizer: Arc<Sanitizer>,
}

impl ToolExecutionQueue {
    /// Construct a queue with a custom `base_ctx`, concurrency cap, and
    /// sanitizer. 默认 sanitizer = `Sanitizer::with_defaults()`(10 个
    /// 默认 pattern 启用)。
    pub fn new(
        registry: Arc<ToolRegistry>,
        hook_engine: Arc<HookEngine>,
        base_ctx: ToolContext,
        max_concurrency: usize,
    ) -> Self {
        let max = if max_concurrency == 0 {
            DEFAULT_MAX_CONCURRENCY
        } else {
            max_concurrency
        };
        Self {
            registry,
            hook_engine,
            base_ctx,
            semaphore: Arc::new(Semaphore::new(max)),
            sanitizer: Arc::new(Sanitizer::with_defaults()),
        }
    }

    /// Convenience: build a queue with sensible defaults (5-way concurrency,
    /// no session/turn metadata, default sanitizer enabled)。
    pub fn with_defaults(
        registry: Arc<ToolRegistry>,
        hook_engine: Arc<HookEngine>,
        base_ctx: ToolContext,
    ) -> Self {
        Self::new(registry, hook_engine, base_ctx, DEFAULT_MAX_CONCURRENCY)
    }

    /// 用自定义 sanitizer 构造 queue (escape hatch,主要给 bootstrap /
    /// 集成测试用)。日常使用 `with_defaults` 即可。
    pub fn with_sanitizer(
        registry: Arc<ToolRegistry>,
        hook_engine: Arc<HookEngine>,
        base_ctx: ToolContext,
        sanitizer: Arc<Sanitizer>,
    ) -> Self {
        let mut q = Self::with_defaults(registry, hook_engine, base_ctx);
        q.sanitizer = sanitizer;
        q
    }

    pub fn hook_engine(&self) -> &Arc<HookEngine> {
        &self.hook_engine
    }

    /// Register a hook on the queue's shared engine. M6 example API.
    pub fn register_hook<H: reflect_hooks::Hook + 'static>(&self, hook: H) {
        self.hook_engine.register(hook);
    }

    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    /// Run a batch of tool calls. Order of returned results matches the
    /// input order — even for the concurrent safe subset, since
    /// `join_all` preserves argument order.
    pub async fn execute_all(&self, calls: Vec<ToolCallRequest>) -> Vec<ToolResult> {
        self.execute_all_with_gate(calls, None).await
    }

    /// M6: run a batch with an optional `ApprovalGate`. When `Some`, tools
    /// with `required_permission() == Prompt` route through the gate; the
    /// gate may also be invoked by `HookDecision::Ask` from a `PreToolUse`
    /// hook. With `None` no approval is requested (back-compat with
    /// `execute_all` and the headless `reflect-exec` driver).
    pub async fn execute_all_with_gate(
        &self,
        calls: Vec<ToolCallRequest>,
        gate: Option<Arc<ApprovalGate>>,
    ) -> Vec<ToolResult> {
        // Partition by concurrency safety. We need to keep the original
        // index so the final result order matches the input order.
        let mut indexed_safe: Vec<(usize, ToolCallRequest)> = Vec::new();
        let mut indexed_unsafe: Vec<(usize, ToolCallRequest)> = Vec::new();
        for (i, c) in calls.into_iter().enumerate() {
            match self.registry.get(&c.name) {
                Some(t) if t.is_concurrency_safe() => indexed_safe.push((i, c)),
                _ => indexed_unsafe.push((i, c)),
            }
        }

        // Unsafe: serial, in original order.
        let mut unsafe_results: Vec<(usize, ToolResult)> = Vec::new();
        for (idx, call) in indexed_unsafe {
            let r = self.execute_single(call, gate.as_ref()).await;
            unsafe_results.push((idx, r));
        }

        // Safe: concurrent via join_all.
        let safe_futures = indexed_safe.into_iter().map(|(idx, call)| {
            let gate_ref = gate.as_ref();
            async move {
                let r = self.execute_single(call, gate_ref).await;
                (idx, r)
            }
        });
        let safe_results: Vec<(usize, ToolResult)> = join_all(safe_futures).await;

        // Merge in original order.
        let mut all: Vec<(usize, ToolResult)> = unsafe_results;
        all.extend(safe_results);
        all.sort_by_key(|(i, _)| *i);
        all.into_iter().map(|(_, r)| r).collect()
    }

    /// Run a single tool call: PreToolUse → (optional ApprovalGate) →
    /// execute → PostToolUse / PostToolUseFailure.
    async fn execute_single(
        &self,
        call: ToolCallRequest,
        gate: Option<&Arc<ApprovalGate>>,
    ) -> ToolResult {
        // Bound concurrency.
        let _permit = match self.semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                return ToolResult {
                    call_id: call.id,
                    content: vec![ContentBlock::text("queue closed")],
                    is_error: true,
                    elapsed_ms: 0,
                    metadata: serde_json::json!({}),
                };
            }
        };

        let tool = self.registry.get(&call.name);
        let tool = match tool {
            Some(t) => t,
            None => {
                return ToolResult {
                    call_id: call.id,
                    content: vec![ContentBlock::text(format!("tool not found: {}", call.name))],
                    is_error: true,
                    elapsed_ms: 0,
                    metadata: serde_json::json!({"error": "tool_not_found"}),
                };
            }
        };

        // Build per-call context.
        let mut ctx = self.base_ctx.clone();
        ctx.call_id = call.id.clone();
        // v1.0.0-rc1+:queue 拿到 gate 后,顺手 inject 到 `ToolContext.approval`,
        // 让 tool 内部能主动调 `gate.ask_tool(...)` 触发 per-action approval
        // modal(典型:同 tool 不同 action 走不同权限)。`base_ctx.approval` 为 `None`
        // 时保持 `None` —— backward-compat。
        if ctx.approval.is_none() {
            ctx.approval = gate.cloned();
        }
        let effective_timeout = if ctx.timeout.is_zero() {
            DEFAULT_TIMEOUT
        } else {
            ctx.timeout
        };

        // PreToolUse hook.
        let hook_ctx = HookContext {
            session_id: ctx.session_id,
            turn_id: ctx.turn_id,
            workspace: ctx.workspace_path(),
            permission_mode: ctx.permission_mode,
        };
        let pre_decision = self
            .hook_engine
            .dispatch(&HookEvent::PreToolUse {
                tool: call.name.clone(),
                args: call.args.clone(),
                ctx: hook_ctx,
            })
            .await;

        // Apply decision.
        let mut effective_args = call.args.clone();
        // Hook-issued Ask reason; routed through the gate (if present)
        // after we settle the simple variants. Set by either the Ask arm or
        // a Combined branch that includes Ask.
        let mut hook_ask_reason: Option<String> = None;
        match pre_decision {
            reflect_hooks::HookDecision::Allow => {}
            reflect_hooks::HookDecision::Deny { reason } => {
                return ToolResult {
                    call_id: call.id,
                    content: vec![ContentBlock::text(format!("Denied by hook: {reason}"))],
                    is_error: true,
                    elapsed_ms: 0,
                    metadata: serde_json::json!({"hook": "deny"}),
                };
            }
            reflect_hooks::HookDecision::Ask { reason } => {
                hook_ask_reason = Some(reason);
            }
            reflect_hooks::HookDecision::ModifyArgs(new_args) => {
                effective_args = new_args;
            }
            reflect_hooks::HookDecision::PermissionOverride(mode) => {
                ctx.permission_mode = mode;
            }
            reflect_hooks::HookDecision::InjectMessage(m) => {
                // Append the reminder to metadata so the model sees it on
                // the next call. (M3 v0: stash in metadata; M4 may move
                // to a proper system-reminder channel.)
                let prev = std::mem::replace(
                    &mut ctx.metadata,
                    serde_json::Value::Object(Default::default()),
                );
                let mut obj = prev.as_object().cloned().unwrap_or_default();
                obj.insert(
                    "system_reminder".into(),
                    serde_json::Value::String(m.content),
                );
                ctx.metadata = serde_json::Value::Object(obj);
            }
            reflect_hooks::HookDecision::Combined(parts) => {
                // Apply each leaf in order. Deny still wins.
                let merged =
                    reflect_hooks::HookEngine::merge(vec![reflect_hooks::HookDecision::Combined(
                        parts,
                    )]);
                match merged {
                    reflect_hooks::HookDecision::Deny { reason } => {
                        return ToolResult {
                            call_id: call.id,
                            content: vec![ContentBlock::text(format!("Denied by hook: {reason}"))],
                            is_error: true,
                            elapsed_ms: 0,
                            metadata: serde_json::json!({"hook": "deny"}),
                        };
                    }
                    reflect_hooks::HookDecision::Ask { reason } => {
                        hook_ask_reason = Some(reason);
                    }
                    reflect_hooks::HookDecision::ModifyArgs(new_args) => {
                        effective_args = new_args;
                    }
                    reflect_hooks::HookDecision::InjectMessage(m) => {
                        let mut obj = ctx.metadata.as_object().cloned().unwrap_or_default();
                        obj.insert(
                            "system_reminder".into(),
                            serde_json::Value::String(m.content),
                        );
                        ctx.metadata = serde_json::Value::Object(obj);
                    }
                    _ => {}
                }
            }
        }

        // M6: approval gate. Two triggers:
        //   1. The tool itself requires `Prompt` permission and the user
        //      hasn't whitelisted it for the session.
        //   2. A `PreToolUse` hook returned `HookDecision::Ask`.
        // With no gate (headless `reflect-exec`), both are no-ops — the
        // tool runs without prompting, matching pre-M6 behaviour.
        //
        // v1.0.0-rc1+:多 action tool (`ast` / `lsp`) 走 per-action 路由
        // `tool.action_permission(&effective_args)`,默认 fallback 到
        // `tool.required_permission()`。
        if let Some(gate_ref) = gate {
            let effective_perm = tool.action_permission(&effective_args);
            let tool_requires_prompt = matches!(effective_perm, PermissionMode::Prompt)
                && !gate_ref.is_session_allowed(&call.name);
            let risk = tool_risk_level(&call.name, &effective_args);
            // P1 bash-classifier:Safe 命令在 Auto 模式下跳过审批。
            let skip_bash_auto_safe = call.name == "bash"
                && ctx.permission_mode == PermissionMode::Auto
                && effective_args
                    .get("cmd")
                    .and_then(|v| v.as_str())
                    .is_some_and(|cmd| classify_command(cmd) == BashCommandClass::Safe);
            if !skip_bash_auto_safe && (tool_requires_prompt || hook_ask_reason.is_some()) {
                let decision = if let Some(reason) = hook_ask_reason.clone() {
                    // Hook-level approval — render under `ApprovalKind::Hook`.
                    gate_ref
                        .ask_hook("pre_tool_use", reason, risk, &ctx.cancel)
                        .await
                } else {
                    gate_ref
                        .ask_tool(&call.name, &effective_args, risk, &ctx.cancel)
                        .await
                };
                match decision {
                    ReviewDecision::Approve => {}
                    ReviewDecision::ApproveForSession => {
                        gate_ref.allow_for_session(&call.name);
                    }
                    ReviewDecision::Deny { reason } => {
                        return ToolResult {
                            call_id: call.id,
                            content: vec![ContentBlock::text(format!("Approval denied: {reason}"))],
                            is_error: true,
                            elapsed_ms: 0,
                            metadata: serde_json::json!({"approval": "denied"}),
                        };
                    }
                }
            }
        }

        // Execute with timeout.
        let start = Instant::now();
        let exec = tool.execute(ctx, effective_args);
        let outcome = timeout(effective_timeout, exec).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        match outcome {
            Ok(Ok(mut output)) => {
                let is_err = output.is_error;
                // v1.0.0-rc2:在 `output_clone` 之前做密钥脱敏 —— 这样
                // PostToolUse 钩子(LangfuseTracker / tracing 等)只看到
                // 脱敏后版本,密钥不会泄露到 rollout / tracing span。
                // 错误 / 超时分支合成的 `ContentBlock::text(...)` 不二次
                // 扫描 —— 那是工具错误模板,不携带真实密钥。
                //
                // Review 2026-06-30 P0-3:此分支不看 `output.is_error` —
                // 工具合法返回 `ToolOutput { is_error: true, content: real_data }`
                // (典型:Bash exit 0 但 stdout 自标 error) 仍走 sanitize。
                // 这是有意设计:`is_error` 是工具语义信号,不等于"内容
                // 可信",密钥检测对所有真实 payload 都应当生效。仅当
                // 外层 `Result::Err(_)` 或 timeout 触发 `Ok(Err(_))` /
                // `Err(_)` 分支(行 434-486)时跳过 sanitize —— 那里合成
                // 的错误模板文本不含真实工具数据。
                sanitize_output(&mut output, &self.sanitizer);
                let output_clone = output.clone();
                let _ = self
                    .hook_engine
                    .dispatch(&HookEvent::PostToolUse {
                        tool: call.name.clone(),
                        result: output_clone,
                        elapsed_ms,
                    })
                    .await;
                ToolResult {
                    call_id: call.id,
                    content: output.content,
                    is_error: is_err,
                    elapsed_ms,
                    metadata: output.metadata,
                }
            }
            Ok(Err(e)) => {
                let tool_error = e.clone();
                let output = ToolOutput {
                    content: vec![ContentBlock::text(e.to_string())],
                    is_error: true,
                    metadata: serde_json::json!({"kind": format!("{e:?}")}),
                    elapsed_ms,
                };
                let _ = self
                    .hook_engine
                    .dispatch(&HookEvent::PostToolUseFailure {
                        tool: call.name.clone(),
                        error: tool_error,
                        elapsed_ms,
                    })
                    .await;
                ToolResult {
                    call_id: call.id,
                    content: output.content,
                    is_error: true,
                    elapsed_ms,
                    metadata: output.metadata,
                }
            }
            Err(_elapsed) => {
                warn!(tool = %call.name, "tool execution timed out");
                let tool_error = ToolError::Timeout { elapsed_ms };
                let output = ToolOutput {
                    content: vec![ContentBlock::text(format!(
                        "tool '{}' timed out after {elapsed_ms}ms",
                        call.name
                    ))],
                    is_error: true,
                    metadata: serde_json::json!({"timeout_ms": elapsed_ms}),
                    elapsed_ms,
                };
                let _ = self
                    .hook_engine
                    .dispatch(&HookEvent::PostToolUseFailure {
                        tool: call.name.clone(),
                        error: tool_error,
                        elapsed_ms,
                    })
                    .await;
                ToolResult {
                    call_id: call.id,
                    content: output.content,
                    is_error: true,
                    elapsed_ms,
                    metadata: output.metadata,
                }
            }
        }
    }
}

/// 按工具名与参数解析审批风险等级;bash 走 [`classify_command`] 动态映射。
fn tool_risk_level(tool_name: &str, args: &Value) -> RiskLevel {
    if tool_name == "bash" {
        if let Some(cmd) = args.get("cmd").and_then(|v| v.as_str()) {
            return classify_command(cmd).risk_level();
        }
    }
    RiskLevel::Medium
}

// Touch unused symbols to avoid dead-code warnings while the queue's
// surrounding wiring (ToolContext.metadata) lands.
#[allow(dead_code)]
fn _suppress_unused() {
    let _: Option<TurnId> = None;
    let _: fn(&ContentBlock) -> bool = |c| matches!(c, ContentBlock::Text { .. });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Tool, ToolContext, ToolError, ToolOutput};
    use async_trait::async_trait;
    use reflect_hooks::HookEngine;
    #[allow(unused_imports)] // pre-M5
    use reflect_protocol::{PermissionMode, ThreadId};
    use std::sync::Arc;

    struct StubTool {
        name: String,
        safe: bool,
    }
    #[async_trait]
    impl Tool for StubTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn is_concurrency_safe(&self) -> bool {
            self.safe
        }
        async fn execute(&self, _ctx: ToolContext, _args: Value) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                content: vec![ContentBlock::text(format!("from {}", self.name))],
                is_error: false,
                metadata: serde_json::json!({}),
                elapsed_ms: 0,
            })
        }
    }

    fn ctx() -> ToolContext {
        ToolContext::for_workspace(".")
    }

    #[tokio::test]
    async fn empty_batch_returns_empty() {
        let reg = Arc::new(ToolRegistry::default());
        let engine = Arc::new(HookEngine::new());
        let q = ToolExecutionQueue::with_defaults(reg, engine, ctx());
        let out = q.execute_all(vec![]).await;
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn unknown_tool_returns_error_result() {
        let reg = Arc::new(ToolRegistry::default());
        let engine = Arc::new(HookEngine::new());
        let q = ToolExecutionQueue::with_defaults(reg, engine, ctx());
        let out = q
            .execute_all(vec![ToolCallRequest {
                id: "1".into(),
                name: "missing".into(),
                args: serde_json::json!({}),
            }])
            .await;
        assert_eq!(out.len(), 1);
        assert!(out[0].is_error);
        assert!(
            out[0]
                .content
                .iter()
                .any(|c| matches!(c, ContentBlock::Text { text } if text.contains("not found")))
        );
    }

    #[tokio::test]
    async fn single_tool_runs_and_returns_result() {
        let reg = Arc::new(ToolRegistry::default());
        reg.register(Arc::new(StubTool {
            name: "stub".into(),
            safe: true,
        }));
        let engine = Arc::new(HookEngine::new());
        let q = ToolExecutionQueue::with_defaults(reg, engine, ctx());
        let out = q
            .execute_all(vec![ToolCallRequest {
                id: "1".into(),
                name: "stub".into(),
                args: serde_json::json!({}),
            }])
            .await;
        assert_eq!(out.len(), 1);
        assert!(!out[0].is_error);
        assert!(matches!(&out[0].content[0], ContentBlock::Text { text } if text == "from stub"));
    }

    #[tokio::test]
    async fn results_preserve_original_order() {
        let reg = Arc::new(ToolRegistry::default());
        reg.register(Arc::new(StubTool {
            name: "a".into(),
            safe: true,
        }));
        reg.register(Arc::new(StubTool {
            name: "b".into(),
            safe: true,
        }));
        let engine = Arc::new(HookEngine::new());
        let q = ToolExecutionQueue::with_defaults(reg, engine, ctx());
        let out = q
            .execute_all(vec![
                ToolCallRequest {
                    id: "1".into(),
                    name: "a".into(),
                    args: serde_json::json!({}),
                },
                ToolCallRequest {
                    id: "2".into(),
                    name: "b".into(),
                    args: serde_json::json!({}),
                },
            ])
            .await;
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].call_id, "1");
        assert_eq!(out[1].call_id, "2");
    }

    /// v1.0.0-rc2:StubTool 输出含假 AWS key,经过 queue 走完后应是脱敏后版本。
    #[tokio::test]
    async fn test_sanitize_in_queue_path() {
        use crate::sanitize::Sanitizer;
        use reflect_protocol::ToolOutput;

        struct LeakyStub;
        #[async_trait]
        impl Tool for LeakyStub {
            fn name(&self) -> &str {
                "leaky"
            }
            fn description(&self) -> &str {
                "emits a fake AWS access key"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _ctx: ToolContext,
                _args: Value,
            ) -> Result<ToolOutput, ToolError> {
                Ok(ToolOutput {
                    content: vec![ContentBlock::text("AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE")],
                    is_error: false,
                    metadata: serde_json::json!({}),
                    elapsed_ms: 0,
                })
            }
        }

        let reg = Arc::new(ToolRegistry::default());
        reg.register(Arc::new(LeakyStub));
        let engine = Arc::new(HookEngine::new());
        let q = ToolExecutionQueue::with_sanitizer(
            reg,
            engine,
            ctx(),
            Arc::new(Sanitizer::with_defaults()),
        );
        let out = q
            .execute_all(vec![ToolCallRequest {
                id: "1".into(),
                name: "leaky".into(),
                args: serde_json::json!({}),
            }])
            .await;
        assert_eq!(out.len(), 1);
        match &out[0].content[0] {
            ContentBlock::Text { text } => {
                // KEY_ASSIGN 优先 AWS pattern(因 AWS 在 KEY_ASSIGN 之前),
                // AWS_ACCESS_KEY 已把 AKIAIOSFODNN7EXAMPLE 整段替换为
                // `[REDACTED:aws_key]`。
                assert!(
                    text.contains("[REDACTED:aws_key]"),
                    "expected AWS marker in: {text}"
                );
                assert!(
                    !text.contains("AKIAIOSFODNN7EXAMPLE"),
                    "raw AWS key must not appear: {text}"
                );
            }
            other => panic!("expected Text block, got {other:?}"),
        }
    }
}
