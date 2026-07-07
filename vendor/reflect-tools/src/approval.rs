//! `ApprovalGate` — per-turn approval routing for tools that require
//! `PermissionMode::Prompt`.
//!
//! Flow (M6):
//! 1. `ToolExecutionQueue::execute_single` checks the tool's
//!    `required_permission`. If `Prompt` and the user hasn't whitelisted the
//!    tool for the session, the queue calls [`ApprovalGate::ask_tool`].
//! 2. `ask_tool` mints a fresh `request_id`, registers a `tokio::sync::oneshot`
//!    waiter, and emits `EventMsg::ApprovalRequest` on the per-turn event
//!    channel.
//! 3. The TUI (or any client) handles the event and sends
//!    `Op::ToolApproval { id: request_id, decision }`.
//! 4. `submission_loop` looks up the gate by `sub_id`, calls
//!    [`ApprovalGate::complete`], which fulfills the oneshot.
//! 5. `ask_tool` returns the `ReviewDecision`. The queue acts on it.
//!
//! `tokio::select!` integrates cancellation: a cancelled token short-circuits
//! the wait with `ReviewDecision::Deny`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use reflect_protocol::{
    ApprovalKind, ApprovalRequestEvent, AskUserAnswer, AskUserInputEvent, Event, EventMsg,
    PermissionBubbleEvent, PermissionMode, Question, ReviewDecision, RiskLevel, ToolError,
    question::AskUserQuestionEvent,
};

/// Map of pending approval `request_id` → oneshot waiter.
pub type ApprovalWaiters = Arc<Mutex<HashMap<String, oneshot::Sender<ReviewDecision>>>>;

/// Map of pending ask-user-question `request_id` → oneshot waiter carrying
/// the structured `AskUserAnswer` back to `AskUserQuestionTool::execute`.
///
/// 与 `ApprovalWaiters` 平行但**独立**(不混用同一个 map,避免 `ReviewDecision` /
/// `AskUserAnswer` 类型擦除带来的混乱)。由 `submission_loop` 持全局共享,
/// per-turn `ApprovalGate` 通过 `with_state(..., Some(qw.clone()))` 共享引用;
/// `Op::AskUserQuestionResponse` 由 `complete_ask_user_question` 全局函数路由。
pub type AskUserQuestionWaiters = Arc<Mutex<HashMap<String, oneshot::Sender<AskUserAnswer>>>>;

/// Map of pending `ask_user` `request_id` → oneshot waiter carrying
/// 用户自由文本回 `AskUserTool::execute`。
pub type AskUserInputWaiters = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

/// v1.2 review P2:bug-2:`ask_user` 工具 prompt 字节上限。LLM 误发
/// 超过此长度的字符串在 `ApprovalGate::ask_user` 入口直接 `InvalidArgs`
/// 拒绝,避免 event channel 撑爆 + TUI 渲染无滚动卡死。
///
/// 16 KiB 远超实际问答 prompt(常见 1 KiB 以内),又能 OOM 之前熔断。
pub const MAX_ASK_USER_PROMPT_BYTES: usize = 16 * 1024;

pub fn complete_ask_user_input(
    waiters: &AskUserInputWaiters,
    request_id: &str,
    text: String,
) -> bool {
    let sender_opt = waiters.lock().remove(request_id);
    match sender_opt {
        Some(tx) => tx.send(text).is_ok(),
        None => {
            warn!(
                request_id = %request_id,
                "ask_user completion arrived but no waiter registered (cancelled?)"
            );
            false
        }
    }
}

/// Resolve a pending ask-user-question directly on the shared waiter map.
/// Used by `submission_loop` so it doesn't have to find the originating gate
/// for each incoming `Op::AskUserQuestionResponse` — request_ids are uuids
/// and globally unique within a session.
pub fn complete_ask_user_question(
    waiters: &AskUserQuestionWaiters,
    request_id: &str,
    answers: AskUserAnswer,
) -> bool {
    let sender_opt = waiters.lock().remove(request_id);
    match sender_opt {
        Some(tx) => tx.send(answers).is_ok(),
        None => {
            warn!(
                request_id = %request_id,
                "ask_user_question completion arrived but no waiter registered (cancelled?)"
            );
            false
        }
    }
}

/// Resolve a pending approval directly on the shared waiter map. Used by
/// `submission_loop` so it doesn't have to find the originating gate for
/// each incoming `Op::ToolApproval` — request_ids are uuids and globally
/// unique within a session.
pub fn complete_approval(
    waiters: &ApprovalWaiters,
    request_id: &str,
    decision: ReviewDecision,
) -> bool {
    let sender_opt = waiters.lock().remove(request_id);
    match sender_opt {
        Some(tx) => tx.send(decision).is_ok(),
        None => {
            warn!(
                request_id = %request_id,
                "approval completion arrived but no waiter is registered (already cancelled?)"
            );
            false
        }
    }
}

/// Per-turn approval gate. Owned by `submission_loop` and handed to
/// `ToolExecutionQueue::execute_all_with_gate`。
pub struct ApprovalGate {
    event_tx: mpsc::Sender<Event>,
    sub_id: String,
    waiters: ApprovalWaiters,
    /// Tools the user has approved for the lifetime of this session.
    session_allow: Arc<Mutex<HashSet<String>>>,
    /// S5a:可选的 permission resolver,`ask_tool` 入口先查规则短路
    /// Allow / Deny。`None` 时退回原 modal-only 行为(向后兼容)。
    permission_resolver: Option<Arc<dyn reflect_permissions::PermissionResolver>>,
    /// v1.1.0 P1 #14:per-gate waiter map for `ask_user_question` 工具的
    /// 结构化问答。每个 `ApprovalGate` 自带一份,`submission_loop` 通过
    /// `Arc<ApprovalGate>` 调 `complete_question` 路由 `Op::AskUserQuestionResponse`
    /// 回执。
    question_waiters: AskUserQuestionWaiters,
    /// v1.1.0 P1 #15:`ask_user` 自由文本询问 waiter map。
    user_input_waiters: AskUserInputWaiters,
    /// 会话级 `PermissionMode`(AcceptEdits / Bubble 短路用)。
    session_permission_mode: Option<Arc<parking_lot::RwLock<PermissionMode>>>,
}

// 手写 Debug —— `Arc<dyn PermissionResolver>` 没 derive Debug。
impl std::fmt::Debug for ApprovalGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApprovalGate")
            .field("sub_id", &self.sub_id)
            .field("waiters", &self.waiters)
            .field("session_allow", &self.session_allow)
            .field(
                "permission_resolver",
                &self
                    .permission_resolver
                    .as_ref()
                    .map(|_| "<dyn PermissionResolver>"),
            )
            .field("question_waiters", &self.question_waiters)
            .field("user_input_waiters", &self.user_input_waiters)
            .field(
                "session_permission_mode",
                &self
                    .session_permission_mode
                    .as_ref()
                    .map(|_| "<RwLock<PermissionMode>>"),
            )
            .finish()
    }
}

impl ApprovalGate {
    pub fn new(event_tx: mpsc::Sender<Event>, sub_id: impl Into<String>) -> Self {
        Self::with_state(
            event_tx,
            sub_id,
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashSet::new())),
            None,
            None,
            None,
            None,
        )
    }

    /// Construct a gate that shares an existing "approved for session" set
    /// with sibling turns (so an `ApproveForSession` decision in turn N
    /// carries over to turn N+1). Waiters are private to this gate.
    pub fn with_session_allow(
        event_tx: mpsc::Sender<Event>,
        sub_id: impl Into<String>,
        session_allow: Arc<Mutex<HashSet<String>>>,
    ) -> Self {
        Self::with_state(
            event_tx,
            sub_id,
            Arc::new(Mutex::new(HashMap::new())),
            session_allow,
            None,
            None,
            None,
            None,
        )
    }

    /// Construct a gate with shared `waiters` and `session_allow`. This
    /// lets `submission_loop` resolve incoming `Op::ToolApproval` against
    /// the same waiter map any gate writes to (request_ids are uuids so
    /// collisions across turns are impossible).
    ///
    /// S5a:第 5 参数 `permission_resolver` 可选。`Some(r)` 时 `ask_tool`
    /// 入口先查 `r.resolve(tool)` 短路 Allow / Deny;`None` 时回退到
    /// 原 modal 行为。**不**修改 `ask_tool` 以外的现有路径语义。
    ///
    /// v1.1.0 P1 #14:第 6 参数 `question_waiters` 可选。`Some(qw)` 时 gate
    /// 把 `ask_user_question` 工具的 oneshot waiter 注册到共享 map(由
    /// `submission_loop` 持全局,`Op::AskUserQuestionResponse` 路由用);
    /// `None` 时 per-gate 新建一个独立 map(向后兼容 + 简单场景)。
    ///
    /// v1.1.0 P1 #15:第 7 参数 `user_input_waiters` —— `ask_user` 工具用。
    /// 第 8 参数 `session_permission_mode` —— AcceptEdits / Bubble 短路。
    #[allow(clippy::too_many_arguments)]
    pub fn with_state(
        event_tx: mpsc::Sender<Event>,
        sub_id: impl Into<String>,
        waiters: ApprovalWaiters,
        session_allow: Arc<Mutex<HashSet<String>>>,
        permission_resolver: Option<Arc<dyn reflect_permissions::PermissionResolver>>,
        question_waiters: Option<AskUserQuestionWaiters>,
        user_input_waiters: Option<AskUserInputWaiters>,
        session_permission_mode: Option<Arc<parking_lot::RwLock<PermissionMode>>>,
    ) -> Self {
        Self {
            event_tx,
            sub_id: sub_id.into(),
            waiters,
            session_allow,
            permission_resolver,
            question_waiters: question_waiters
                .unwrap_or_else(|| Arc::new(Mutex::new(HashMap::new()))),
            user_input_waiters: user_input_waiters
                .unwrap_or_else(|| Arc::new(Mutex::new(HashMap::new()))),
            session_permission_mode,
        }
    }

    pub fn sub_id(&self) -> &str {
        &self.sub_id
    }

    pub fn waiters(&self) -> &ApprovalWaiters {
        &self.waiters
    }

    /// v1.1.0 P1 #14:per-gate waiter map for `ask_user_question` 工具。
    pub fn question_waiters(&self) -> &AskUserQuestionWaiters {
        &self.question_waiters
    }

    /// v1.1.0 P1 #15:per-gate waiter map for `ask_user` 工具。
    pub fn user_input_waiters(&self) -> &AskUserInputWaiters {
        &self.user_input_waiters
    }

    pub fn session_allow_handle(&self) -> Arc<Mutex<HashSet<String>>> {
        self.session_allow.clone()
    }

    pub fn is_session_allowed(&self, tool_name: &str) -> bool {
        self.session_allow.lock().contains(tool_name)
    }

    pub fn allow_for_session(&self, tool_name: impl Into<String>) {
        self.session_allow.lock().insert(tool_name.into());
    }

    /// Fulfill a pending approval. Called by `submission_loop` when an
    /// `Op::ToolApproval` or `Op::HookApproval` arrives. Returns `true` if a
    /// matching waiter was found and resolved.
    pub fn complete(&self, request_id: &str, decision: ReviewDecision) -> bool {
        complete_approval(&self.waiters, request_id, decision)
    }

    /// v1.1.0 P1 #14:per-gate 包装的 `complete_ask_user_question`(per-gate
    /// `question_waiters` 模式)。`submission_loop` 默认走全局
    /// `complete_ask_user_question` 函数(共享 map),tool 端调试时可用
    /// 这个 method。返回 `true` 表示找到 waiter 并成功发送。
    pub fn complete_question(&self, request_id: &str, answers: AskUserAnswer) -> bool {
        complete_ask_user_question(&self.question_waiters, request_id, answers)
    }

    /// Ask the user to approve a tool call. Emits `ApprovalRequest` and waits
    /// on a oneshot until the matching `Op::ToolApproval` arrives, the cancel
    /// token fires, or the event channel closes.
    ///
    /// S5a:入口先查 `permission_resolver`(若挂载):
    /// - `RuleMatch::Allow` → 直接返回 `Approve`(**不**写 session_allow,
    ///   一次 tool call 的 allow 不该污染"session 持久"语义;
    ///   真正想"session 内总是允许"用 `/permissions allow <tool>` 持久规则)。
    /// - `RuleMatch::Deny` → 直接返回 `Deny { reason }`。
    /// - `RuleMatch::Ask` 或 `NoMatch` → 走正常 modal 流程(后者退化)。
    ///
    /// 短路优先于 session_allow 缓存 + 用户 modal 三段决策链 —— 显式规则
    /// 比"上次点了 Always Allow"更权威。
    pub async fn ask_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        risk: RiskLevel,
        cancel: &CancellationToken,
    ) -> ReviewDecision {
        // 1. S5a:resolver 短路。Ask / NoMatch / None → fall through。
        let mut force_modal = false;
        if let Some(resolver) = &self.permission_resolver {
            match resolver.resolve(tool_name).await {
                reflect_permissions::RuleMatch::Allow => {
                    return ReviewDecision::Approve;
                }
                reflect_permissions::RuleMatch::Deny => {
                    return ReviewDecision::Deny {
                        reason: format!("denied by rule for {tool_name}"),
                    };
                }
                reflect_permissions::RuleMatch::Ask => {
                    force_modal = true;
                }
                reflect_permissions::RuleMatch::NoMatch => {}
            }
        }

        // 2. 会话 `PermissionMode` 短路(AcceptEdits / Bubble / Deny)。
        // 显式 `RuleMatch::Ask` 规则优先,不覆盖。
        if !force_modal {
            if let Some(mode_ref) = &self.session_permission_mode {
                let mode = *mode_ref.read();
                if mode == PermissionMode::Deny {
                    return ReviewDecision::Deny {
                        reason: "session permission mode is deny".into(),
                    };
                }
                if mode.auto_approves_tool(tool_name) {
                    if mode == PermissionMode::Bubble {
                        self.emit_permission_bubble(tool_name, args, risk).await;
                    }
                    return ReviewDecision::Approve;
                }
            }
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<ReviewDecision>();
        self.waiters.lock().insert(request_id.clone(), tx);

        let ev = Event::new(
            self.sub_id.clone(),
            EventMsg::ApprovalRequest(ApprovalRequestEvent {
                request_id: request_id.clone(),
                kind: ApprovalKind::Tool {
                    tool_name: tool_name.to_string(),
                    args: args.clone(),
                },
                risk,
            }),
        );

        if self.event_tx.send(ev).await.is_err() {
            // Receiver dropped — no consumer, can't get an answer. Auto-deny.
            self.waiters.lock().remove(&request_id);
            return ReviewDecision::Deny {
                reason: "approval channel closed".into(),
            };
        }

        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                self.waiters.lock().remove(&request_id);
                ReviewDecision::Deny { reason: "cancelled".into() }
            }
            res = rx => {
                match res {
                    Ok(decision) => decision,
                    Err(_) => ReviewDecision::Deny {
                        reason: "approval channel dropped".into(),
                    },
                }
            }
        }
    }

    /// Ask the user to approve a hook decision (mirror of `ask_tool` for
    /// `ApprovalKind::Hook`). Wired through `HookDecision::Ask` when hooks
    /// gain that variant in M6.
    pub async fn ask_hook(
        &self,
        hook_name: &str,
        decision_preview: impl Into<String>,
        risk: RiskLevel,
        cancel: &CancellationToken,
    ) -> ReviewDecision {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<ReviewDecision>();
        self.waiters.lock().insert(request_id.clone(), tx);

        let ev = Event::new(
            self.sub_id.clone(),
            EventMsg::ApprovalRequest(ApprovalRequestEvent {
                request_id: request_id.clone(),
                kind: ApprovalKind::Hook {
                    hook_name: hook_name.to_string(),
                    decision_preview: decision_preview.into(),
                },
                risk,
            }),
        );

        if self.event_tx.send(ev).await.is_err() {
            self.waiters.lock().remove(&request_id);
            return ReviewDecision::Deny {
                reason: "approval channel closed".into(),
            };
        }

        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                self.waiters.lock().remove(&request_id);
                ReviewDecision::Deny { reason: "cancelled".into() }
            }
            res = rx => res.unwrap_or(ReviewDecision::Deny {
                reason: "approval channel dropped".into(),
            }),
        }
    }

    /// v1.1.0 P1 #14:LLM 主动向用户发起 1-4 道结构化问题。emit
    /// `EventMsg::AskUserQuestion`,oneshot 等待 `Op::AskUserQuestionResponse`
    /// 回执或 cancel。
    ///
    /// # Errors
    ///
    /// - `ToolError::InvalidArgs`:`questions` 为空 / 超过 4 道 / 某道
    ///   `header` 超过 12 字符 / 某道 `options` 数量不在 [2,4] 范围。
    /// - `ToolError::Cancelled`:`cancel` token 在等待响应时触发。
    /// - `ToolError::Execution`:event channel 已关闭,无法 emit。
    ///
    /// # Flow
    ///
    /// 1. 校验 `questions`(数量、每道 `header` / `options` 边界)。
    /// 2. 分配 `request_id` (uuid),oneshot channel 写入 `question_waiters`。
    /// 3. emit `EventMsg::AskUserQuestion` 给 TUI / headless 客户端。
    /// 4. `tokio::select!` 等回执 / cancel / channel 关闭。
    /// 5. 返回 `AskUserAnswer`(可能为空 — 用户按 Esc 取消)。
    pub async fn ask_question(
        &self,
        questions: Vec<Question>,
        cancel: &CancellationToken,
    ) -> Result<AskUserAnswer, ToolError> {
        // 1. 校验 questions 数量。
        if questions.is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "ask_user_question: 'questions' must not be empty".into(),
            });
        }
        if questions.len() > reflect_protocol::question::MAX_QUESTIONS {
            return Err(ToolError::InvalidArgs {
                message: format!(
                    "ask_user_question: too many questions ({} > {})",
                    questions.len(),
                    reflect_protocol::question::MAX_QUESTIONS
                ),
            });
        }
        // 2. 校验每道 question(把 `Question::new` 的 Result 转 ToolError)。
        for (i, q) in questions.iter().enumerate() {
            if q.options.len() < reflect_protocol::question::MIN_OPTIONS
                || q.options.len() > reflect_protocol::question::MAX_OPTIONS
            {
                return Err(ToolError::InvalidArgs {
                    message: format!(
                        "ask_user_question: question[{i}] options.len() = {} (must be in [{}, {}])",
                        q.options.len(),
                        reflect_protocol::question::MIN_OPTIONS,
                        reflect_protocol::question::MAX_OPTIONS
                    ),
                });
            }
            if q.header.chars().count() > reflect_protocol::question::MAX_HEADER_CHARS {
                return Err(ToolError::InvalidArgs {
                    message: format!(
                        "ask_user_question: question[{i}] header too long ({} > {})",
                        q.header.chars().count(),
                        reflect_protocol::question::MAX_HEADER_CHARS
                    ),
                });
            }
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<AskUserAnswer>();
        self.question_waiters.lock().insert(request_id.clone(), tx);

        let ev = Event::new(
            self.sub_id.clone(),
            EventMsg::AskUserQuestion(AskUserQuestionEvent::new(request_id.clone(), questions)),
        );

        if self.event_tx.send(ev).await.is_err() {
            self.question_waiters.lock().remove(&request_id);
            return Err(ToolError::Execution(
                "ask_user_question: event channel closed".into(),
            ));
        }

        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                self.question_waiters.lock().remove(&request_id);
                Err(ToolError::Cancelled)
            }
            res = rx => match res {
                Ok(answers) => Ok(answers),
                Err(_) => Err(ToolError::Execution(
                    "ask_user_question: waiter dropped before response".into(),
                )),
            }
        }
    }

    /// Bubble 模式:emit 非阻塞通知,不等待用户决策。
    async fn emit_permission_bubble(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        risk: RiskLevel,
    ) {
        let preview = args
            .as_object()
            .map(|o| {
                o.iter()
                    .take(3)
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty());
        let ev = Event::new(
            self.sub_id.clone(),
            EventMsg::PermissionBubble(PermissionBubbleEvent {
                tool_name: tool_name.to_string(),
                args_preview: preview,
                risk,
            }),
        );
        let _ = self.event_tx.send(ev).await;
    }

    /// v1.1.0 P1 #15:LLM 向用户发起自由文本询问。emit `AskUserInput`,
    /// oneshot 等待 `Op::AskUserInputResponse` 或 cancel。
    ///
    /// # 超时(v1.2 review P1:bug-1)
    ///
    /// `timeout_secs` 控制最大等待秒数;`0` 表示永不超时(由 cancel token
    /// 或 event channel 关闭兜底,适合 `request_human_input` 这类持久化
    /// 等待场景)。`> 0` 时 `tokio::time::sleep` 与 cancel / rx 一起
    /// `tokio::select!`;超时到达返回
    /// `ToolError::Execution("ask_user: timed out after N seconds")`,
    /// 同步清理 `user_input_waiters` 防止泄漏。
    ///
    /// # 权限短路(v1.2 review P1:bug-2)
    ///
    /// `tool_name` 用于 resolver 查询(`ask_user` / `request_human_input`)。
    /// - resolver `Deny` rule → `InvalidArgs`(显式规则赢)。
    /// - session `PermissionMode::Deny` → `InvalidArgs`。
    /// - session `PermissionMode::Bubble` → `InvalidArgs`(无法对自由文本
    ///   做"自动同意",保守退化为 Deny)。
    /// - resolver `Allow` rule → **不**短路(自由文本没有合法默认值,
    ///   必须由用户显式输入;该规则对 `ask_user` 无效)。
    ///
    /// # Prompt 长度(v1.2 review P2:bug-2)
    ///
    /// `prompt` 字节数超 [`MAX_ASK_USER_PROMPT_BYTES`] 直接拒;防止
    /// LLM 误发 100 MB 字符串挤爆 event channel + TUI 渲染。
    pub async fn ask_user(
        &self,
        tool_name: &str,
        prompt: impl Into<String>,
        cancel: &CancellationToken,
        timeout_secs: u64,
    ) -> Result<String, ToolError> {
        let prompt = prompt.into();
        if prompt.trim().is_empty() {
            return Err(ToolError::InvalidArgs {
                message: "ask_user: 'prompt' must not be empty".into(),
            });
        }
        if prompt.len() > MAX_ASK_USER_PROMPT_BYTES {
            return Err(ToolError::InvalidArgs {
                message: format!(
                    "ask_user: 'prompt' too long ({} > {} bytes)",
                    prompt.len(),
                    MAX_ASK_USER_PROMPT_BYTES
                ),
            });
        }

        // v1.2 review P1:bug-2:权限短路(镜像 ask_tool:294-378)。
        // 1. resolver 短路:`Deny` 立即拒绝;`Allow` 不短路(自由文本无默认)。
        if let Some(resolver) = &self.permission_resolver {
            if let reflect_permissions::RuleMatch::Deny = resolver.resolve(tool_name).await {
                return Err(ToolError::InvalidArgs {
                    message: format!("ask_user: denied by rule for {tool_name}"),
                });
            }
        }
        // 2. session `PermissionMode` 短路:`Deny` / `Bubble` 拒绝。
        if let Some(mode_ref) = &self.session_permission_mode {
            let mode = *mode_ref.read();
            if mode == PermissionMode::Deny {
                return Err(ToolError::InvalidArgs {
                    message: "ask_user: session permission mode is deny".into(),
                });
            }
            if mode == PermissionMode::Bubble {
                return Err(ToolError::InvalidArgs {
                    message:
                        "ask_user: session permission mode is bubble (no auto-answer for free text)"
                            .into(),
                });
            }
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<String>();
        self.user_input_waiters
            .lock()
            .insert(request_id.clone(), tx);

        let ev = Event::new(
            self.sub_id.clone(),
            EventMsg::AskUserInput(AskUserInputEvent::new(request_id.clone(), prompt)),
        );

        if self.event_tx.send(ev).await.is_err() {
            self.user_input_waiters.lock().remove(&request_id);
            return Err(ToolError::Execution(
                "ask_user: event channel closed".into(),
            ));
        }

        // v1.2 review P1:bug-1:把"rx 收到用户响应"包装成 Result,避免
        // `match res { ... }` 在 `tokio::select!` handler 里的复杂模式让
        // macro parser 困惑。
        let rx_result: Result<String, String> = if timeout_secs == 0 {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err("cancelled".into()),
                recv = rx => recv.map_err(|_| "dropped".into()),
            }
        } else {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err("cancelled".into()),
                recv = rx => recv.map_err(|_| "dropped".into()),
                _sleep = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                    Err(format!("timeout:{timeout_secs}"))
                }
            }
        };

        match rx_result {
            Ok(text) => Ok(text),
            Err(reason) if reason == "cancelled" => {
                self.user_input_waiters.lock().remove(&request_id);
                Err(ToolError::Cancelled)
            }
            Err(reason) if reason == "dropped" => Err(ToolError::Execution(
                "ask_user: waiter dropped before response".into(),
            )),
            Err(reason) if reason.starts_with("timeout:") => {
                self.user_input_waiters.lock().remove(&request_id);
                Err(ToolError::Execution(format!(
                    "ask_user: timed out after {timeout_secs} seconds"
                )))
            }
            Err(other) => Err(ToolError::Execution(format!(
                "ask_user: unexpected wait outcome: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    fn make_gate() -> (ApprovalGate, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(8);
        (ApprovalGate::new(tx, "sub-1"), rx)
    }

    #[tokio::test]
    async fn ask_tool_emits_event_and_waits_for_decision() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();

        // Spawn the ask; it will block on the oneshot.
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker = tokio::spawn(async move {
            g.ask_tool(
                "bash",
                &serde_json::json!({"cmd": "ls"}),
                RiskLevel::Medium,
                &cancel_c,
            )
            .await
        });

        // Read the emitted event and extract the request_id.
        let ev = timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let request_id = match ev.msg {
            EventMsg::ApprovalRequest(req) => {
                assert!(matches!(req.kind, ApprovalKind::Tool { .. }));
                assert_eq!(req.risk, RiskLevel::Medium);
                req.request_id
            }
            other => panic!("expected ApprovalRequest, got {other:?}"),
        };

        // Complete the approval.
        assert!(gate_arc.complete(&request_id, ReviewDecision::Approve));

        let decision = timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(decision, ReviewDecision::Approve);
    }

    #[tokio::test]
    async fn cancel_short_circuits_to_deny() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();

        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker = tokio::spawn(async move {
            g.ask_tool("bash", &serde_json::json!({}), RiskLevel::Low, &cancel_c)
                .await
        });

        // Give the asker a moment to register.
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();

        let decision = timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(decision, ReviewDecision::Deny { .. }));
        // Waiter should have been removed.
        assert!(gate_arc.waiters().lock().is_empty());
    }

    #[tokio::test]
    async fn complete_unknown_request_returns_false() {
        let (gate, _rx) = make_gate();
        assert!(!gate.complete("nope", ReviewDecision::Approve));
    }

    #[tokio::test]
    async fn closed_event_channel_auto_denies() {
        let (tx, rx) = mpsc::channel::<Event>(8);
        drop(rx);
        let gate = ApprovalGate::new(tx, "sub-x");
        let cancel = CancellationToken::new();
        let d = gate
            .ask_tool("bash", &serde_json::json!({}), RiskLevel::Low, &cancel)
            .await;
        assert!(matches!(d, ReviewDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn session_allow_persists() {
        let (gate, _rx) = make_gate();
        assert!(!gate.is_session_allowed("bash"));
        gate.allow_for_session("bash");
        assert!(gate.is_session_allowed("bash"));
    }

    // ── S5a:permission_resolver 短路测试 ────────────────────────────────
    //
    // 验证 ask_tool 入口查 resolver:
    // - Allow → 直接 Approve(不 emit ApprovalRequest event,不调 modal)。
    // - Deny  → 直接 Deny { reason }。
    // - NoMatch → 走原 modal 流程(emit event 等用户决策)。
    // - None resolver → 同 NoMatch(向后兼容)。
    //
    // 用 `reflect_permissions::InMemoryPermissionStore` + `StorePermissionResolver`
    // 构造真实 store 链;测试短路路径需要 `CancellationToken` 不被 cancel。

    use reflect_permissions::{
        InMemoryPermissionStore, PermissionAction, PermissionResolver, PermissionRule,
        StorePermissionResolver,
    };
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    /// 构造挂载 resolver 的 gate(共享 in-memory store)。
    fn make_gate_with_resolver(
        rules: Vec<(&str, PermissionAction)>,
    ) -> (ApprovalGate, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(8);
        let store: Arc<dyn reflect_permissions::PermissionStore> =
            Arc::new(InMemoryPermissionStore::new());
        // sync 闭包注入 rules;InMemoryPermissionStore::add 是 async,需
        // 走 block_on 把 rules 灌进 store。但 tests 已经是 #[tokio::test],
        // 用 spawn_blocking 不便 —— 直接 futures::executor::block_on。
        for (tool, action) in rules {
            let rule = PermissionRule {
                tool: tool.into(),
                action,
                tool_glob: None,
                shell_pattern: None,
            };
            futures::executor::block_on(store.add(rule)).unwrap();
        }
        let resolver: Arc<dyn PermissionResolver> = Arc::new(StorePermissionResolver::new(store));
        let waiters: ApprovalWaiters = Arc::new(Mutex::new(HashMap::new()));
        let session_allow: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        (
            ApprovalGate::with_state(
                tx,
                "sub-r",
                waiters,
                session_allow,
                Some(resolver),
                None,
                None,
                None,
            ),
            rx,
        )
    }

    /// `Allow` rule 短路 → 直接 Approve,**不**emit ApprovalRequest event
    ///(否则下游 `rx` 收到 event 但我们期望 Approve 是终态)。
    #[tokio::test]
    async fn ask_tool_allow_rule_short_circuits_to_approve() {
        let (gate, mut rx) = make_gate_with_resolver(vec![("Bash", PermissionAction::Allow)]);
        let cancel = CancellationToken::new();
        let decision = gate
            .ask_tool("Bash", &serde_json::json!({}), RiskLevel::Low, &cancel)
            .await;
        assert_eq!(decision, ReviewDecision::Approve);
        // rx 应该空(没 emit event),因为短路直接返回。
        assert!(
            rx.try_recv().is_err(),
            "should not emit event on Allow short-circuit"
        );
    }

    /// `Deny` rule 短路 → 直接 Deny { reason },**不**emit event。
    #[tokio::test]
    async fn ask_tool_deny_rule_short_circuits_to_deny() {
        let (gate, mut rx) = make_gate_with_resolver(vec![("Write", PermissionAction::Deny)]);
        let cancel = CancellationToken::new();
        let decision = gate
            .ask_tool("Write", &serde_json::json!({}), RiskLevel::Medium, &cancel)
            .await;
        match decision {
            ReviewDecision::Deny { reason } => {
                assert!(
                    reason.contains("Write"),
                    "reason should mention tool: {reason}"
                );
                assert!(reason.contains("denied by rule"), "got: {reason}");
            }
            other => panic!("expected Deny, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "should not emit event on Deny short-circuit"
        );
    }

    /// `NoMatch`(resolver 存在但 tool 无规则)→ 走原 modal 流程,emit event。
    /// 验证方法:cancel token,期望 ask 返回 Deny { cancelled }。
    #[tokio::test]
    async fn ask_tool_no_match_falls_through_to_modal() {
        let (gate, _rx) = make_gate_with_resolver(vec![("Bash", PermissionAction::Allow)]);
        // 查询 "Write" → store 里没规则 → NoMatch → fall through 到 modal。
        let cancel = CancellationToken::new();
        let cancel_for_spawn = cancel.clone();
        let ask = tokio::spawn(async move {
            gate.ask_tool(
                "Write",
                &serde_json::json!({}),
                RiskLevel::Medium,
                &cancel_for_spawn,
            )
            .await
        });
        // 短暂等让 ask 跑进 modal 等待。
        tokio::time::sleep(Duration::from_millis(20)).await;
        // cancel token 让 modal 短路返回 Deny { cancelled }。
        cancel.cancel();
        let decision = ask.await.unwrap();
        assert!(matches!(decision, ReviewDecision::Deny { .. }));
    }

    /// `None` resolver(向后兼容路径)→ 走原 modal 流程。
    #[tokio::test]
    async fn ask_tool_no_resolver_falls_through_to_modal() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        let cancel_for_spawn = cancel.clone();
        let ask = tokio::spawn(async move {
            gate.ask_tool(
                "Bash",
                &serde_json::json!({}),
                RiskLevel::Low,
                &cancel_for_spawn,
            )
            .await
        });
        // 短暂等让 ask 跑进 modal 等待。
        tokio::time::sleep(Duration::from_millis(20)).await;
        // cancel 让 ask 返回。
        cancel.cancel();
        let decision = ask.await.unwrap();
        assert!(matches!(decision, ReviewDecision::Deny { .. }));
    }

    // ── v1.1.0 P1 #14:ask_question 测试 ─────────────────────────────────
    //
    // 覆盖:
    // 1. happy path:emit event + 等回执 + 拿回答案。
    // 2. cancel:等待被取消 → ToolError::Cancelled。
    // 3. event channel 关闭 → ToolError::Execution。
    // 4. validation:空 questions / 超过 4 道 / options 越界 / header 超长。
    // 5. complete_question 找不到 waiter → false。
    // 6. multi-question flow:3 题 / multi_select / Other 自定义。

    use reflect_protocol::question::{Answer, AskUserAnswer, Question, QuestionOption};

    fn sample_questions() -> Vec<Question> {
        vec![
            Question {
                header: "Lang".into(),
                question: "Pick a language".into(),
                options: vec![
                    QuestionOption {
                        label: "Rust".into(),
                        description: "safe + fast".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "Go".into(),
                        description: "simple".into(),
                        preview: None,
                    },
                ],
                multi_select: false,
            },
            Question {
                header: "Deploy".into(),
                question: "Where to deploy?".into(),
                options: vec![
                    QuestionOption {
                        label: "AWS".into(),
                        description: "managed".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "GCP".into(),
                        description: "managed".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "Self".into(),
                        description: "BYO infra".into(),
                        preview: None,
                    },
                ],
                multi_select: true,
            },
        ]
    }

    #[tokio::test]
    async fn ask_question_emits_event_and_waits_for_response() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);

        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker =
            tokio::spawn(async move { g.ask_question(sample_questions(), &cancel_c).await });

        // 取出 emit 的 event,验证 AskUserQuestion + request_id 是 uuid。
        let ev = timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let request_id = match ev.msg {
            EventMsg::AskUserQuestion(e) => {
                assert_eq!(e.questions.len(), 2);
                assert!(!e.questions[0].multi_select);
                assert!(e.questions[1].multi_select);
                e.request_id
            }
            other => panic!("expected AskUserQuestion, got {other:?}"),
        };
        // uuid 格式:8-4-4-4-12。
        assert_eq!(request_id.len(), 36, "got: {request_id}");

        // 回执:用户答了 2 道题(其中第 1 道用 "Other" 自定义文本)。
        let answers = AskUserAnswer {
            answers: vec![
                Answer::single(0).with_custom("prefer async runtime"),
                Answer::multi(vec![0, 2]),
            ],
        };
        assert!(gate_arc.complete_question(&request_id, answers.clone()));

        let result = timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result.answers.len(), 2);
        assert_eq!(result.answers[0].selected, vec![0]);
        assert_eq!(
            result.answers[0].custom.as_deref(),
            Some("prefer async runtime")
        );
        assert_eq!(result.answers[1].selected, vec![0, 2]);
    }

    #[tokio::test]
    async fn ask_question_cancelled_returns_cancelled_error() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker =
            tokio::spawn(async move { g.ask_question(sample_questions(), &cancel_c).await });

        // 让 ask 跑进 modal 等待。
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();

        let err = timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, ToolError::Cancelled), "got: {err:?}");
        // waiter 已被清理。
        assert!(gate_arc.question_waiters().lock().is_empty());
    }

    #[tokio::test]
    async fn ask_question_closed_event_channel_returns_execution_error() {
        let (tx, rx) = mpsc::channel::<Event>(8);
        drop(rx);
        let gate = ApprovalGate::new(tx, "sub-x");
        let cancel = CancellationToken::new();
        let err = gate
            .ask_question(sample_questions(), &cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::Execution(_)), "got: {err:?}");
    }

    #[tokio::test]
    async fn ask_question_rejects_empty_questions() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        let err = gate.ask_question(vec![], &cancel).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[tokio::test]
    async fn ask_question_rejects_too_many_questions() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        let five_questions: Vec<Question> = (0..5)
            .map(|i| Question {
                header: format!("Q{i}"),
                question: format!("question {i}"),
                options: vec![
                    QuestionOption {
                        label: "A".into(),
                        description: "a".into(),
                        preview: None,
                    },
                    QuestionOption {
                        label: "B".into(),
                        description: "b".into(),
                        preview: None,
                    },
                ],
                multi_select: false,
            })
            .collect();
        let err = gate
            .ask_question(five_questions, &cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[tokio::test]
    async fn ask_question_rejects_invalid_options_count() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        // 1 个 option — 越下界。
        let bad = vec![Question {
            header: "H".into(),
            question: "Q".into(),
            options: vec![QuestionOption {
                label: "A".into(),
                description: "a".into(),
                preview: None,
            }],
            multi_select: false,
        }];
        let err = gate.ask_question(bad, &cancel).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[tokio::test]
    async fn ask_question_rejects_header_too_long() {
        let (gate, _rx) = make_gate();
        let cancel = CancellationToken::new();
        let long = "a".repeat(20);
        let bad = vec![Question {
            header: long,
            question: "Q".into(),
            options: vec![
                QuestionOption {
                    label: "A".into(),
                    description: "a".into(),
                    preview: None,
                },
                QuestionOption {
                    label: "B".into(),
                    description: "b".into(),
                    preview: None,
                },
            ],
            multi_select: false,
        }];
        let err = gate.ask_question(bad, &cancel).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[tokio::test]
    async fn complete_question_unknown_request_returns_false() {
        let (gate, _rx) = make_gate();
        assert!(!gate.complete_question("nope", AskUserAnswer::empty(1)));
    }

    #[tokio::test]
    async fn ask_question_user_pressed_esc_yields_empty_answers() {
        // 用户在 TUI modal 按 Esc → submission_loop 收到 Op::AskUserQuestionResponse
        // 带空 `AskUserAnswer` → `gate.complete_question` 路由回 waiter。
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker =
            tokio::spawn(async move { g.ask_question(sample_questions(), &cancel_c).await });

        let ev = timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let request_id = match ev.msg {
            EventMsg::AskUserQuestion(e) => e.request_id,
            other => panic!("expected AskUserQuestion, got {other:?}"),
        };

        // 模拟用户按 Esc:回执空答案(answers.len() == questions.len())。
        let empty = AskUserAnswer::empty(2);
        assert!(gate_arc.complete_question(&request_id, empty));

        let result = timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result.answers.len(), 2);
        for ans in &result.answers {
            assert!(ans.selected.is_empty());
            assert!(ans.custom.is_none());
        }
    }

    fn make_gate_with_permission_mode(
        mode: PermissionMode,
    ) -> (ApprovalGate, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(8);
        let pm = Arc::new(parking_lot::RwLock::new(mode));
        (
            ApprovalGate::with_state(
                tx,
                "sub-pm",
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashSet::new())),
                None,
                None,
                None,
                Some(pm),
            ),
            rx,
        )
    }

    #[tokio::test]
    async fn accept_edits_short_circuits_edit_tools_only() {
        let (gate, mut rx) = make_gate_with_permission_mode(PermissionMode::AcceptEdits);
        let cancel = CancellationToken::new();
        let edit = gate
            .ask_tool(
                "write",
                &serde_json::json!({"path": "a"}),
                RiskLevel::Medium,
                &cancel,
            )
            .await;
        assert_eq!(edit, ReviewDecision::Approve);
        assert!(rx.try_recv().is_err());

        let bash = tokio::spawn({
            let gate = gate;
            let cancel = cancel.clone();
            async move {
                gate.ask_tool("bash", &serde_json::json!({}), RiskLevel::Medium, &cancel)
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
        assert!(matches!(bash.await.unwrap(), ReviewDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn bubble_mode_emits_event_and_auto_approves() {
        let (gate, mut rx) = make_gate_with_permission_mode(PermissionMode::Bubble);
        let cancel = CancellationToken::new();
        let decision = gate
            .ask_tool("bash", &serde_json::json!({}), RiskLevel::Low, &cancel)
            .await;
        assert_eq!(decision, ReviewDecision::Approve);
        let ev = rx.recv().await.unwrap();
        assert!(matches!(ev.msg, EventMsg::PermissionBubble(_)));
    }

    #[tokio::test]
    async fn ask_user_emits_event_and_returns_text() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let asker =
            tokio::spawn(async move { g.ask_user("ask_user", "Your name?", &cancel, 0).await });

        let ev = rx.recv().await.unwrap();
        let request_id = match ev.msg {
            EventMsg::AskUserInput(e) => {
                assert_eq!(e.prompt, "Your name?");
                e.request_id
            }
            other => panic!("expected AskUserInput, got {other:?}"),
        };

        assert!(complete_ask_user_input(
            gate_arc.user_input_waiters(),
            &request_id,
            "Alice".into()
        ));

        let text = asker.await.unwrap().unwrap();
        assert_eq!(text, "Alice");
    }

    // ── v1.2 review P1:bug-1:ask_user timeout 路径 ──────────────────────
    //
    // 覆盖:
    // 1. timeout_secs = 0:永不超时,cancel token 才能终结等待。
    // 2. timeout_secs > 0:超时到达返回 ToolError::Execution,waiter 被清理
    //    (后续 complete 同 id 返回 false)。
    // 3. timeout_secs > 0:在超时到达前用户响应 → 正常返回,优先级高于 timeout。

    #[tokio::test]
    async fn ask_user_timeout_zero_waits_forever_until_cancel() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_for_ask = cancel.clone();
        let asker = tokio::spawn(async move {
            g.ask_user("ask_user", "Long running?", &cancel_for_ask, 0)
                .await
        });

        // 拿出 emit 的 event 表明 ask 已就位。
        let ev = rx.recv().await.unwrap();
        let _ = match ev.msg {
            EventMsg::AskUserInput(e) => e.request_id,
            other => panic!("expected AskUserInput, got {other:?}"),
        };

        // 等 200ms 确认 ask 没有 timeout 自杀。
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !asker.is_finished(),
            "ask_user(0) should not self-terminate"
        );

        // cancel 终结。
        cancel.cancel();
        let err = asker.await.unwrap().unwrap_err();
        assert!(matches!(err, ToolError::Cancelled), "got: {err:?}");
        assert!(gate_arc.user_input_waiters().lock().is_empty());
    }

    #[tokio::test]
    async fn ask_user_timeout_fires_execution_error_and_clears_waiter() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let asker = tokio::spawn(async move { g.ask_user("ask_user", "Soon?", &cancel, 1).await });

        let ev = rx.recv().await.unwrap();
        let request_id = match ev.msg {
            EventMsg::AskUserInput(e) => e.request_id,
            other => panic!("expected AskUserInput, got {other:?}"),
        };

        // 等超过 1s 让 timeout 兜底触发。
        let err = tokio::time::timeout(Duration::from_secs(3), asker)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        match err {
            ToolError::Execution(msg) => {
                assert!(msg.contains("timed out"), "got: {msg}");
                assert!(msg.contains("1"), "got: {msg}");
            }
            other => panic!("expected Execution timeout error, got: {other:?}"),
        }
        // waiter 已被清理,二次 complete 返回 false。
        assert!(gate_arc.user_input_waiters().lock().is_empty());
        assert!(!complete_ask_user_input(
            gate_arc.user_input_waiters(),
            &request_id,
            "late".into()
        ));
    }

    #[tokio::test]
    async fn ask_user_response_wins_over_timeout() {
        let (gate, mut rx) = make_gate();
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let asker = tokio::spawn(async move { g.ask_user("ask_user", "Race?", &cancel, 5).await });

        let ev = rx.recv().await.unwrap();
        let request_id = match ev.msg {
            EventMsg::AskUserInput(e) => e.request_id,
            other => panic!("expected AskUserInput, got {other:?}"),
        };

        // 50ms 内响应,远早于 5s timeout。
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(complete_ask_user_input(
            gate_arc.user_input_waiters(),
            &request_id,
            "fast".into()
        ));

        let text = tokio::time::timeout(Duration::from_secs(2), asker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(text, "fast");
    }

    // ── v1.2 review P1:bug-2:ask_user 权限短路 ──────────────────────────
    //
    // 覆盖:
    // 1. resolver Deny rule → InvalidArgs,不发 event。
    // 2. resolver Allow rule → 不短路(自由文本无默认值),正常发 event。
    // 3. session PermissionMode::Deny → InvalidArgs。
    // 4. session PermissionMode::Bubble → InvalidArgs(无法自动回答)。
    // 5. resolver NoMatch + 无 mode → 走原 modal 流程。

    /// 构造挂载 resolver 的 gate(共享 in-memory store)。
    fn make_ask_user_gate_with_resolver(
        rules: Vec<(&str, PermissionAction)>,
    ) -> (ApprovalGate, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(8);
        let store: Arc<dyn reflect_permissions::PermissionStore> =
            Arc::new(InMemoryPermissionStore::new());
        for (tool, action) in rules {
            let rule = PermissionRule {
                tool: tool.into(),
                action,
                tool_glob: None,
                shell_pattern: None,
            };
            futures::executor::block_on(store.add(rule)).unwrap();
        }
        let resolver: Arc<dyn PermissionResolver> = Arc::new(StorePermissionResolver::new(store));
        (
            ApprovalGate::with_state(
                tx,
                "sub-au",
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashSet::new())),
                Some(resolver),
                None,
                None,
                None,
            ),
            rx,
        )
    }

    /// 构造挂载 session permission mode 的 gate(无 resolver)。
    fn make_ask_user_gate_with_mode(mode: PermissionMode) -> (ApprovalGate, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel::<Event>(8);
        let pm = Arc::new(parking_lot::RwLock::new(mode));
        (
            ApprovalGate::with_state(
                tx,
                "sub-aum",
                Arc::new(Mutex::new(HashMap::new())),
                Arc::new(Mutex::new(HashSet::new())),
                None,
                None,
                None,
                Some(pm),
            ),
            rx,
        )
    }

    #[tokio::test]
    async fn ask_user_resolver_deny_short_circuits_to_invalid_args() {
        let (gate, mut rx) =
            make_ask_user_gate_with_resolver(vec![("ask_user", PermissionAction::Deny)]);
        let cancel = CancellationToken::new();
        let err = gate
            .ask_user("ask_user", "Q?", &cancel, 0)
            .await
            .unwrap_err();
        match err {
            ToolError::InvalidArgs { message } => {
                assert!(message.contains("denied by rule"), "got: {message}");
                assert!(message.contains("ask_user"), "got: {message}");
            }
            other => panic!("expected InvalidArgs, got: {other:?}"),
        }
        // 不发 event。
        assert!(rx.try_recv().is_err(), "should not emit event on Deny");
    }

    #[tokio::test]
    async fn ask_user_resolver_allow_does_not_short_circuit() {
        // 契约:`Allow` rule 不短路(自由文本无默认值)。验证方法:发 event,
        // cancel token 终结等待,确认走到了 modal 流程。
        let (gate, _rx) =
            make_ask_user_gate_with_resolver(vec![("ask_user", PermissionAction::Allow)]);
        let cancel = CancellationToken::new();
        let gate_arc = Arc::new(gate);
        let g = gate_arc.clone();
        let cancel_c = cancel.clone();
        let asker = tokio::spawn(async move { g.ask_user("ask_user", "Q?", &cancel_c, 0).await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
        let err = asker.await.unwrap().unwrap_err();
        // 走 modal 流程 → cancel 终结 → Cancelled,不是 InvalidArgs。
        assert!(matches!(err, ToolError::Cancelled), "got: {err:?}");
    }

    #[tokio::test]
    async fn ask_user_session_mode_deny_short_circuits() {
        let (gate, mut rx) = make_ask_user_gate_with_mode(PermissionMode::Deny);
        let cancel = CancellationToken::new();
        let err = gate
            .ask_user("ask_user", "Q?", &cancel, 0)
            .await
            .unwrap_err();
        match err {
            ToolError::InvalidArgs { message } => {
                assert!(message.contains("deny"), "got: {message}");
            }
            other => panic!("expected InvalidArgs, got: {other:?}"),
        }
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn ask_user_session_mode_bubble_short_circuits() {
        // Bubble 模式无法对自由文本做"自动同意"——保守退化为 Deny。
        let (gate, mut rx) = make_ask_user_gate_with_mode(PermissionMode::Bubble);
        let cancel = CancellationToken::new();
        let err = gate
            .ask_user("ask_user", "Q?", &cancel, 0)
            .await
            .unwrap_err();
        match err {
            ToolError::InvalidArgs { message } => {
                assert!(message.contains("bubble"), "got: {message}");
            }
            other => panic!("expected InvalidArgs, got: {other:?}"),
        }
        assert!(rx.try_recv().is_err());
    }
}
