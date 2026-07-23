//! Sub-types referenced by Submission / Op / EventMsg.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::question::AskUserAnswer;

/// Newtype around a UUID for thread identification. Stable across the lifetime
/// of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub Uuid);

impl ThreadId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype around a UUID for turn identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnId(pub Uuid);

impl TurnId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a UUID string (e.g. from `Op::Rewind.to_turn_id`) into a `TurnId`.
    /// Used by the rewind-truncate path in `submission_loop` to convert the
    /// wire-form `Option<String>` into the typed id the recorder expects.
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for TurnId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TurnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One piece of user input (text / image / skill activation / question answer).
///
/// v1.1.0 P1 #14 新增 `QuestionAnswer` —— 允许用户通过标准 `Op::UserInput`
/// 流回答 `EventMsg::AskUserQuestion`(与 `Op::AskUserQuestionResponse` 平行,
/// 适合 TUI 在 question modal 之外用 input bar 自由输入答案的场景)。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserInputItem {
    Text {
        text: String,
    },
    Image {
        data: Vec<u8>,
        mime_type: String,
    },
    LocalImage {
        path: PathBuf,
    },
    Skill {
        name: String,
        #[serde(default)]
        args: Option<serde_json::Value>,
    },
    /// v1.1.0 P1 #14: 对 LLM 主动询问的结构化回答。`request_id` 与
    /// `EventMsg::AskUserQuestion.request_id` 配对。`None` 表示用户按
    /// Esc 取消(LLM 收到空答案)。
    QuestionAnswer {
        request_id: String,
        answers: AskUserAnswer,
    },
}

/// 工具运行所需的用户信任级别。
///
/// 放在 `reflect-protocol` 中,用于打破 `reflect-tools` (需要在 `ToolSpec::required_permission` 里使用)
/// 与 `reflect-hooks` (需要在 `HookDecision::PermissionOverride` 里使用) 之间的循环依赖。
/// `reflect-hooks` 重新导出此类型。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// 工具直接运行,无需提示(只读工具的默认值)。
    #[default]
    Auto,
    /// 工具需经用户审批才能运行(有副作用工具的默认值)。
    Prompt,
    /// 工具被禁止运行。
    Deny,
    /// Plan mode(v1.x):用户主动进入只读规划阶段,
    /// 任何不在白名单内的工具(包括 bash/edit/write)都会被
    /// `PlanModeGate` hook blanket-deny。
    Plan,
    /// 自动批准文件编辑类工具(`write` / `edit` / `delete`),其余
    /// `Prompt` 工具仍走 approval modal(对齐 Claude Code acceptEdits)。
    AcceptEdits,
    /// 非阻塞 bubble 通知 + 自动批准:emit `PermissionBubble` 事件供
    /// TUI 展示,不弹 blocking modal(对齐 Claude Code bubble)。
    Bubble,
    /// 危险:静默跳过所有工具审批,不弹 modal、不发 bubble(对齐
    /// Claude Code `bypass permissions`)。**不**影响 `ask_user` /
    /// `ask_user_question` 等主动索取人类输入的工具(那些仍会弹出)。
    Bypass,
}

impl PermissionMode {
    /// 人类可读标签(用于日志和 TUI approval modal)。
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::Auto => "auto",
            PermissionMode::Prompt => "prompt",
            PermissionMode::Deny => "deny",
            PermissionMode::Plan => "plan",
            PermissionMode::AcceptEdits => "accept_edits",
            PermissionMode::Bubble => "bubble",
            PermissionMode::Bypass => "bypass",
        }
    }

    /// TUI `/mode` / Tab / Shift+Tab 的循环顺序,对齐 Claude Code 的
    /// `default → accept edits → plan → bypass permissions`。
    /// 循环 = `Auto→AcceptEdits→Plan→Bypass→Auto`。
    /// `Prompt | Deny | Bubble` 不进默认循环:从它们循环会落到 `Auto`
    /// (三者仍经 `/mode <name>` 直达,见 `slash::parse_mode`)。
    pub fn next_in_ui_cycle(self) -> Self {
        match self {
            PermissionMode::Auto => PermissionMode::AcceptEdits,
            PermissionMode::AcceptEdits => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Bypass,
            PermissionMode::Bypass => PermissionMode::Auto,
            PermissionMode::Prompt | PermissionMode::Deny | PermissionMode::Bubble => {
                PermissionMode::Auto
            }
        }
    }

    /// 是否应在 `ApprovalGate::ask_tool` 中自动批准(不弹 modal)。
    pub fn auto_approves_tool(&self, tool_name: &str) -> bool {
        match self {
            PermissionMode::Bubble | PermissionMode::Bypass => true,
            PermissionMode::AcceptEdits => is_edit_tool_name(tool_name),
            _ => false,
        }
    }
}

/// 文件编辑类工具名 —— `AcceptEdits` 模式短路用。
pub fn is_edit_tool_name(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write" | "edit" | "delete" | "Write" | "Edit" | "Delete"
    )
}

/// Plan 模式的会话级唯一标识。Plan 由 `EnterPlanModeTool` 进入、
/// `ExitPlanModeTool` 退出,整个生命周期由 `plan_id` 串联。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlanId(pub Uuid);

impl PlanId {
    /// 生成新的 plan 标识(UUID v4)。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PlanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PlanId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PlanId(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approve,
    Deny { reason: String },
    ApproveForSession,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ThreadSettingsOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<ApprovalPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<SandboxPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_concurrency: Option<usize>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    #[default]
    Auto,
    Prompt,
    Deny,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxPolicy {
    #[default]
    WorkspaceOnly,
    /// M3+ (requires landlock/mac-sandbox).
    OsSandbox,
    /// M3+ (no restrictions).
    FullAccess,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
}

/// v1.x S4:`/effort` slash 命令的协议镜像枚举。
///
/// 之所以**不**直接复用 `reflect_llm::ReasoningEffort`,是因为
/// `reflect-protocol` 不能反向依赖 `reflect-llm`(避免循环依赖 + 协议层
/// 与实现层解耦)。本枚举与 `reflect_llm::ReasoningEffort` 字段一一对应,
/// 由 `reflect-core::submission_loop` 在收到 `Op::SetEffort` 后调
/// `From<ReasoningEffortMirror> for ReasoningEffort` 桥接到 LLM 路径。
///
/// `Default = Low` 与 Anthropic / OpenAI 默认 reasoning 强度一致;`/effort`
/// 不带参数时也按 Low 处理,避免 "未设置 = 不思考" 的歧义。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffortMirror {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionConfiguredEvent {
    pub session_id: ThreadId,
    pub model: String,
    pub provider: String,
    pub approval_policy: ApprovalPolicy,
    pub sandbox_policy: SandboxPolicy,
    /// 引擎报告的模型上下文窗口大小(token),供 TUI 上下文用量条做分母。
    /// 缺省 `None`(未知模型 / 旧 payload),TUI 此时优雅省略上下文条。
    /// 经 `reflect_llm::context_window_for` 回退表补全,避免与 LLM 层漂移。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_size: Option<u32>,
}

/// Output content produced by a tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolOutput {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub metadata: serde_json::Value,
    pub elapsed_ms: u64,
}

/// Tool execution error. Lives in protocol (not `reflect-tools`) so that
/// `reflect-hooks` can carry one in `HookEvent::PostToolUseFailure`
/// without creating a `tools ↔ hooks` cycle.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolError {
    #[error("invalid arguments: {message}")]
    InvalidArgs { message: String },
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("permission denied: {reason}")]
    PermissionDenied { reason: String },
    #[error("timeout after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("path escaped sandbox: {path}")]
    PathEscape { path: std::path::PathBuf },
    #[error("io: {0}")]
    Io(String),
    #[error("hook denied: {reason}")]
    HookDenied { reason: String },
}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> Self {
        ToolError::Io(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        data: Vec<u8>,
        mime_type: String,
    },
    Diff {
        unified_diff: String,
    },
    /// LLM-requested tool invocation (M2+). Surfaced in `latest_content`
    /// after the model call so the graph can decide to dispatch.
    ToolUse {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    /// Result of a tool execution (M2+). Appended to `latest_content` after
    /// the queue runs so the next model call sees the tool output.
    ToolResult {
        call_id: String,
        output: ToolOutput,
    },
}

impl ContentBlock {
    /// Convenience constructor for the most common variant.
    pub fn text(s: impl Into<String>) -> Self {
        ContentBlock::Text { text: s.into() }
    }
}

// ── JsonSchema impls for non-derivable types ────────────────────────────────
//
// schemars 0.8 doesn't impl JsonSchema for `uuid::Uuid`, but the protocol
// uses transparent UUID newtypes (ThreadId / TurnId / PlanId). Implement
// them as string subschemas so they emit `{"type": "string", "format": "uuid"}`.
// The serde-transparent encoding is the same string form, so roundtrip
// stays intact across the JSON Schema → json2ts → TypeScript pipeline.

impl JsonSchema for ThreadId {
    fn schema_name() -> String {
        "ThreadId".to_string()
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        let mut obj = schemars::schema::SchemaObject::default();
        obj.instance_type = Some(schemars::schema::InstanceType::String.into());
        obj.format = Some("uuid".to_string());
        schemars::schema::Schema::Object(obj)
    }
}

impl JsonSchema for TurnId {
    fn schema_name() -> String {
        "TurnId".to_string()
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        let mut obj = schemars::schema::SchemaObject::default();
        obj.instance_type = Some(schemars::schema::InstanceType::String.into());
        obj.format = Some("uuid".to_string());
        schemars::schema::Schema::Object(obj)
    }
}

impl JsonSchema for PlanId {
    fn schema_name() -> String {
        "PlanId".to_string()
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        let mut obj = schemars::schema::SchemaObject::default();
        obj.instance_type = Some(schemars::schema::InstanceType::String.into());
        obj.format = Some("uuid".to_string());
        schemars::schema::Schema::Object(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_id_is_unique() {
        let a = ThreadId::new();
        let b = ThreadId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn user_input_item_text_serde() {
        let item = UserInputItem::Text { text: "hi".into() };
        let j = serde_json::to_string(&item).unwrap();
        assert!(j.contains(r#""type":"text""#), "got: {j}");
    }

    #[test]
    fn user_input_item_question_answer_roundtrip() {
        use crate::question::{Answer, AskUserAnswer};
        let item = UserInputItem::QuestionAnswer {
            request_id: "q-1".into(),
            answers: AskUserAnswer {
                answers: vec![Answer::single(1).with_custom("alt: skip")],
            },
        };
        let j = serde_json::to_string(&item).unwrap();
        assert!(j.contains(r#""type":"question_answer""#), "got: {j}");
        assert!(j.contains(r#""request_id":"q-1""#), "got: {j}");
        let back: UserInputItem = serde_json::from_str(&j).unwrap();
        if let UserInputItem::QuestionAnswer {
            request_id,
            answers,
        } = back
        {
            assert_eq!(request_id, "q-1");
            assert_eq!(answers.answers.len(), 1);
            assert_eq!(answers.answers[0].selected, vec![1]);
            assert_eq!(answers.answers[0].custom.as_deref(), Some("alt: skip"));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn approval_policy_default_is_auto() {
        assert_eq!(ApprovalPolicy::default(), ApprovalPolicy::Auto);
    }

    #[test]
    fn permission_mode_default_is_auto() {
        assert_eq!(PermissionMode::default(), PermissionMode::Auto);
    }

    #[test]
    fn permission_mode_serde_uses_snake_case() {
        let s = serde_json::to_string(&PermissionMode::Prompt).unwrap();
        assert_eq!(s, "\"prompt\"");
        let back: PermissionMode = serde_json::from_str(&s).unwrap();
        assert_eq!(back, PermissionMode::Prompt);
    }

    #[test]
    fn permission_mode_plan_serde_roundtrip() {
        // v1.x: Plan 变体必须能被 serde 序列化与反序列化,且 wire 格式为 `"plan"`。
        let s = serde_json::to_string(&PermissionMode::Plan).unwrap();
        assert_eq!(s, "\"plan\"", "Plan 应序列化为小写字符串");
        let back: PermissionMode = serde_json::from_str(&s).unwrap();
        assert_eq!(back, PermissionMode::Plan);
        assert_eq!(PermissionMode::Plan.as_str(), "plan");
    }

    #[test]
    fn permission_mode_accept_edits_and_bubble_serde_roundtrip() {
        for mode in [PermissionMode::AcceptEdits, PermissionMode::Bubble] {
            let s = serde_json::to_string(&mode).unwrap();
            let back: PermissionMode = serde_json::from_str(&s).unwrap();
            assert_eq!(back, mode);
        }
        assert_eq!(PermissionMode::AcceptEdits.as_str(), "accept_edits");
        assert_eq!(PermissionMode::Bubble.as_str(), "bubble");
    }

    #[test]
    fn permission_mode_bypass_serde_roundtrip() {
        // Bypass 必须能被 serde 序列化与反序列化,且 wire 格式为 `"bypass"`。
        let s = serde_json::to_string(&PermissionMode::Bypass).unwrap();
        assert_eq!(s, "\"bypass\"", "Bypass 应序列化为小写字符串");
        let back: PermissionMode = serde_json::from_str(&s).unwrap();
        assert_eq!(back, PermissionMode::Bypass);
        assert_eq!(PermissionMode::Bypass.as_str(), "bypass");
    }

    #[test]
    fn permission_mode_ui_cycle_is_four_step() {
        // 对齐 Claude Code: Auto → AcceptEdits → Plan → Bypass → Auto
        assert_eq!(
            PermissionMode::Auto.next_in_ui_cycle(),
            PermissionMode::AcceptEdits
        );
        assert_eq!(
            PermissionMode::AcceptEdits.next_in_ui_cycle(),
            PermissionMode::Plan
        );
        assert_eq!(
            PermissionMode::Plan.next_in_ui_cycle(),
            PermissionMode::Bypass
        );
        assert_eq!(
            PermissionMode::Bypass.next_in_ui_cycle(),
            PermissionMode::Auto
        );
    }

    #[test]
    fn permission_mode_ui_cycle_non_default_falls_to_auto() {
        // Prompt / Deny / Bubble 不进默认循环,从它们循环会落到 Auto。
        for m in [
            PermissionMode::Prompt,
            PermissionMode::Deny,
            PermissionMode::Bubble,
        ] {
            assert_eq!(m.next_in_ui_cycle(), PermissionMode::Auto, "from {m:?}");
        }
    }

    #[test]
    fn bypass_auto_approves_all_tools() {
        // Bypass 静默放行所有工具(对齐 Claude Code bypass permissions)。
        for tool in ["bash", "write", "Edit", "delete", "read", "custom"] {
            assert!(
                PermissionMode::Bypass.auto_approves_tool(tool),
                "Bypass should auto-approve {tool}"
            );
        }
    }

    #[test]
    fn session_configured_event_missing_context_window_defaults_none() {
        // 旧 wire payload(无 context_window_size 字段)反序列化时该字段应为 None。
        let j = r#"{
            "session_id": "00000000-0000-0000-0000-000000000000",
            "model": "claude-sonnet-4-latest",
            "provider": "anthropic",
            "approval_policy": "auto",
            "sandbox_policy": "workspace_only"
        }"#;
        let sc: SessionConfiguredEvent = serde_json::from_str(j).unwrap();
        assert_eq!(sc.context_window_size, None);
    }

    #[test]
    fn is_edit_tool_name_matches_write_edit_delete() {
        assert!(is_edit_tool_name("write"));
        assert!(is_edit_tool_name("Edit"));
        assert!(!is_edit_tool_name("bash"));
    }

    #[test]
    fn plan_id_is_unique_and_roundtrips() {
        // PlanId 必须是会话级唯一;透明 newtype 直接序列化底层 UUID。
        let a = PlanId::new();
        let b = PlanId::new();
        assert_ne!(a, b);
        let j = serde_json::to_string(&a).unwrap();
        // `#[serde(transparent)]` 直接输出 UUID 字符串,无额外包装。
        let back: PlanId = serde_json::from_str(&j).unwrap();
        assert_eq!(back, a);
    }
}
