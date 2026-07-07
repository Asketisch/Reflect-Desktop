//! `submission_loop` — consumes `Submission`s, drives turns via the 4-node
//! `StateGraph`, emits events.
//!
//! M2: replaced the M1 inline LLM loop with a real `StateGraph::run` that
//! uses `nodes::model_call` / `nodes::tool_exec` / `nodes::check_stop`.
//! M3: hooks fire through `HookEngine`.
//! M4: `pre_loop` runs compaction + memory + skills + prompt builder
//! before each model call; `NodeContext` carries the M4 deps.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use reflect_hooks::{HookEngine, HookEvent};
use reflect_llm::{ChatMessage, SharedModelRegistry, SystemBlocks};
use reflect_protocol::{
    AbortReason, ContextCompactedEvent, ContextCompactedStrategy, Event, EventMsg, MessageRole,
    PermissionMode, PermissionModeChangedEvent, PlanId, PlanReadyEvent, PlanRejectedEvent,
    PlanRequestEvent, ReasoningEffortMirror, RolloutRecord, RolloutRecorder,
    SessionConfiguredEvent, Submission, TokenCountEvent, TurnAbortedEvent, TurnCompleteEvent,
    TurnId, TurnStartedEvent, TurnStatus, UserInputItem,
};
use reflect_tools::{
    ApprovalGate, ApprovalWaiters, AskUserInputWaiters, AskUserQuestionWaiters, PlanApprovalGate,
    ToolExecutionQueue, ToolRegistry, complete_approval, complete_ask_user_input,
    complete_ask_user_question, complete_plan_approval,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::background_tasks::BackgroundTaskQueue;
use crate::config::{AgentConfig, M4Deps};
use crate::graph::StateGraph;
use crate::graph::state::AgentState;
use crate::steering_queue::SteeringQueue;

/// Cheap-to-clone per-turn context handed to every graph node.
#[derive(Clone)]
pub struct NodeContext {
    pub turn_id: TurnId,
    /// M5: stable thread id for the lifetime of this session. Set once by
    /// `submission_loop` and reused across turns so the rollout recorder
    /// can tag every record with the same session.
    pub session_id: reflect_protocol::ThreadId,
    /// 当前 turn 用的 model spec。v0.2.2 起改为 `Arc<RwLock<String>>`:
    /// 持 `AgentConfig.model` 的共享副本,`model_call` / `cache_break`
    /// 每次读最新值,使 `reflect-exec` 热重载切换 model 时下一个 LLM
    /// 请求立刻走新 model。`Arc` clone 廉价、锁粒度仅 RwLock<String>。
    pub model: Arc<parking_lot::RwLock<String>>,
    /// v1.0 多 Provider 路由:角色 → spec slot 的路由策略。`model_call`
    /// 入口用 `policy.resolve(Role::Main)` 拿到 spec,失败时由
    /// `ModelRegistry::next_for` 在 pool 内自动切下一个 credential。
    pub policy: Arc<reflect_llm::RoutingPolicy>,
    pub registry: SharedModelRegistry,
    pub hook_engine: Arc<HookEngine>,
    pub tools_queue: Arc<ToolExecutionQueue>,
    pub sub_id: String,
    pub cancel: CancellationToken,
    pub event_tx: mpsc::Sender<Event>,
    /// Initial messages for this turn (User input). M4: replaced by
    /// state.messages at the end of `pre_loop`.
    pub messages: Vec<ChatMessage>,
    pub max_iterations: u32,
    /// M4 dependencies. Optional so tests can omit.
    pub m4: Option<M4Deps>,
    /// M5: optional recorder copied off `m4.recorder` so individual nodes
    /// (notably `pre_loop`) can emit Compaction / Message records without
    /// walking through `m4`.
    pub recorder: Option<Arc<dyn RolloutRecorder>>,
    /// M6: per-turn approval gate. `Some` when a TUI / lib client wants
    /// `Prompt`-permission tools to be confirmed; `None` for headless
    /// `reflect-exec`-style yolo execution.
    pub approval_gate: Option<Arc<ApprovalGate>>,
    /// v1.x S4:当前会话的 reasoning effort(`Low` / `Medium` / `High`)。
    /// `model_call` 在入口拍快照构造 `ChatRequest::thinking`;中途
    /// `/effort` 切换影响下一轮而非当前轮。
    pub effort: Arc<parking_lot::RwLock<ReasoningEffortMirror>>,
    /// v1.2 P1-12:会话级累计 token 用量(跨 turn 累加)。`model_call`
    /// 每次调用后累加 `_usage`;预算检查与 `get_context_remaining` 工具读它。
    pub session_usage: Arc<parking_lot::RwLock<reflect_protocol::TokenUsage>>,
    /// v1.2 P1-12:会话级 token 预算硬上限(共享句柄,与 `AgentConfig`
    /// 同一把 RwLock,让热重载贯穿)。`None` = 仅靠 `max_iterations`;
    /// `Some(n)` = 累计 `total_tokens >= n` 时 `model_call` 提前返回 `None`。
    pub token_budget: Arc<parking_lot::RwLock<Option<u64>>>,
    /// v1.2 P1-12(已有-B):`/compact` 强制标志共享句柄。`Op::Compact` 置
    /// `true`,`pre_loop` 读 + 清零,据此强制运行 compactor(无视阈值)。
    pub force_compact_next: Arc<parking_lot::RwLock<bool>>,
}

pub async fn submission_loop(
    mut sub_rx: mpsc::Receiver<Submission>,
    turn_subs: Arc<Mutex<HashMap<String, mpsc::Sender<Event>>>>,
    session_subs: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
    cfg: AgentConfig,
    registry: SharedModelRegistry,
    _tools: Arc<ToolRegistry>,
    tools_queue: Arc<ToolExecutionQueue>,
) {
    // Hook engine is owned by `tools_queue` (M6). We don't construct it
    // here; the queue provides `register_hook` for callers to extend.
    let hook_engine = tools_queue.hook_engine().clone();

    // M6: session-wide approval state. The waiters map is shared across all
    // per-turn gates so request_ids (uuids) route directly without finding
    // the originating gate. The session_allow set persists ApproveForSession
    // decisions across turns.
    let approval_waiters: ApprovalWaiters = Arc::new(Mutex::new(std::collections::HashMap::new()));
    // v1.1.0 P1 #14:session-wide ask-user-question waiters。同 `approval_waiters`
    // 模式 —— per-turn gate 通过 `with_state(..., Some(qw.clone()))` 共享引用,
    // `Op::AskUserQuestionResponse` 进来时由 `complete_ask_user_question` 全局
    // 路由,不需要找原始 gate。
    let question_waiters: AskUserQuestionWaiters =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let user_input_waiters: AskUserInputWaiters =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let session_allow = Arc::new(Mutex::new(std::collections::HashSet::new()));
    // v1.x Plan mode:全局 `PlanApprovalGate` 持有所有 pending plan 切换
    // 请求的 oneshot。`Op::EnterPlanMode` / `Op::ExitPlanMode` 注册 waiter
    // 然后 emit `PlanRequest` / `PlanReady` 等待 TUI 回执;`Op::PlanApproval`
    // 投递决策后由 `complete_plan_approval` 唤醒 waiter。
    let plan_approval_gate = PlanApprovalGate::new();
    // P2:session 级 steering 队列与后台任务注入队列。
    let steering_queue = Arc::new(Mutex::new(SteeringQueue::new()));
    let background_tasks = Arc::new(BackgroundTaskQueue::new());
    // Whether to install per-turn ApprovalGates. M6 v0: opt-in via the
    // `AgentConfig.approvals` flag (set by TUI / lib facade) or via the
    // `REFLECT_APPROVALS=1` env var. Headless `reflect-exec` keeps it
    // off so the existing JSONL path stays yolo by default.
    let approval_enabled = cfg.approvals
        || std::env::var("REFLECT_APPROVALS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    // SessionStart hook fires once per thread.
    let session_id = reflect_protocol::ThreadId::new();
    let _ = hook_engine
        .dispatch(&HookEvent::SessionStart {
            session_id,
            config: serde_json::json!({ "model": cfg.current_model() }),
        })
        .await;

    let mut session_emitted = false;
    while let Some(sub) = sub_rx.recv().await {
        let turn_tx = turn_subs
            .lock()
            .remove(&sub.id)
            .unwrap_or_else(|| mpsc::channel(8).0);

        match sub.op {
            reflect_protocol::Op::UserInput {
                items,
                thread_settings,
            } => {
                // v1.3 analytics:一个 turn 一个根 span,涵盖 pre_loop →
                // model_call → tool_exec → check_stop 全程,OTLP 后端
                // 看到的是嵌套 span 而非离散事件。
                let turn_span = tracing::info_span!(
                    "agent.turn",
                    sub_id = %sub.id,
                );
                let _turn_enter = turn_span.enter();
                // P2:合并 steering 队列中的 NOW / ATTACHMENT 消息。
                let mut merged_items = items;
                {
                    let mut sq = steering_queue.lock();
                    for msg in sq.drain_all() {
                        merged_items.extend(msg.items);
                    }
                }
                // P2:注入已完成的后台任务结果(作为 system 风格文本块)。
                for task in background_tasks.drain_completed() {
                    if let Some(result) = task.result {
                        merged_items.push(UserInputItem::Text {
                            text: format!(
                                "[background task {} ({}) completed]\n{}",
                                task.id, task.kind, result
                            ),
                        });
                    }
                }

                if !session_emitted {
                    let mut sc = SessionConfiguredEvent::new(
                        cfg.current_model(),
                        provider_of(&cfg.current_model()),
                    );
                    // v1.x:填入模型上下文窗口(供 TUI 上下文用量条做分母)。
                    // 引擎侧用同一张 `context_window_for` 表,避免与 LLM 层漂移。
                    sc.context_window_size = reflect_llm::context_window_for(&sc.model);
                    // v1.2 P1-12:同步写入共享句柄,让 `get_context_remaining`
                    // 工具读到同一值(热重载切 model 后此处刷新)。
                    *cfg.context_window_size.write() = sc.context_window_size;
                    let ev = Event::new(
                        reflect_protocol::EVENT_ID_NONE,
                        EventMsg::SessionConfigured(sc),
                    );
                    let _ = turn_tx.send(ev.clone()).await;
                    fan_out_session(&session_subs, &ev);
                    // M5: persist the session header to the rollout recorder.
                    if let Some(rec) = cfg.m4.as_ref().and_then(|m| m.recorder.clone()) {
                        let _ = rec
                            .record(RolloutRecord::SessionMeta {
                                session_id,
                                model: cfg.current_model(),
                                started_at: chrono::Utc::now(),
                            })
                            .await;
                    }
                    session_emitted = true;
                }
                // 共享 `cfg.model` 的 Arc 副本,让后续 graph 节点的每次
                // LLM 调用都读最新值(配合热重载)。`thread_settings.model`
                // 仍是 String,这里做一次 String→Arc<RwLock> 适配。
                let model: Arc<parking_lot::RwLock<String>> = thread_settings
                    .model
                    .map(|s| Arc::new(parking_lot::RwLock::new(s)))
                    .unwrap_or_else(|| Arc::clone(&cfg.model));
                let cancel = cfg.cancel.clone();
                let sub_id = sub.id.clone();
                let turn_id = TurnId::new();
                let _ = turn_tx
                    .send(Event::new(
                        sub_id.clone(),
                        EventMsg::TurnStarted(TurnStartedEvent {
                            turn_id,
                            user_message_id: Some(uuid::Uuid::new_v4().to_string()),
                        }),
                    ))
                    .await;

                // Build the initial message list from the user input.
                let messages: Vec<ChatMessage> = merged_items
                    .into_iter()
                    .filter_map(|i| match i {
                        UserInputItem::Text { text } => {
                            Some(ChatMessage::User(reflect_llm::UserContent {
                                blocks: vec![reflect_llm::ContentBlock::Text { text }],
                            }))
                        }
                        _ => None,
                    })
                    .collect();

                let ctx = NodeContext {
                    turn_id,
                    session_id,
                    model: model.clone(),
                    policy: cfg.policy.clone(),
                    registry: registry.clone(),
                    hook_engine: hook_engine.clone(),
                    tools_queue: tools_queue.clone(),
                    sub_id: sub_id.clone(),
                    cancel: cancel.clone(),
                    event_tx: turn_tx.clone(),
                    messages,
                    max_iterations: 32,
                    m4: cfg.m4.clone(),
                    recorder: cfg.m4.as_ref().and_then(|m| m.recorder.clone()),
                    // v1.x S4:`/effort` 切换的读取源。`Clone` 后多副本共享
                    // 同一把 RwLock,`model_call` 在入口拍快照,单轮 LLM
                    // 调用不受中途 `/effort` 切换影响。
                    effort: cfg.effort.clone(),
                    // v1.2 P1-12:会话级 token 用量 / 预算。与 `cfg` 共享同一把
                    // RwLock(`Arc` clone),`model_call` 写、预算检查 /
                    // get_context_remaining 工具读。
                    session_usage: cfg.session_usage.clone(),
                    token_budget: cfg.token_budget.clone(),
                    force_compact_next: cfg.force_compact_next.clone(),
                    approval_gate: approval_enabled.then(|| {
                        Arc::new(ApprovalGate::with_state(
                            turn_tx.clone(),
                            sub_id.clone(),
                            approval_waiters.clone(),
                            session_allow.clone(),
                            cfg.permission_resolver.clone(),
                            Some(question_waiters.clone()),
                            Some(user_input_waiters.clone()),
                            Some(cfg.permission_mode.clone()),
                        ))
                    }),
                };

                let state = AgentState::default();
                let graph = StateGraph::new(state, ctx);

                // M6: spawn the turn as a tokio task so the submission
                // loop stays free to process Op::ToolApproval /
                // Op::HookApproval / Op::Interrupt / Op::Shutdown
                // submissions while the turn awaits user input through
                // `ApprovalGate`. The previous synchronous `graph.run().await`
                // deadlocked any approval flow.
                let turn_tx_clone = turn_tx.clone();
                let sub_id_clone = sub_id.clone();
                let recorder = cfg.m4.as_ref().and_then(|m| m.recorder.clone());
                tokio::spawn(async move {
                    let final_state = graph.run().await;

                    // v1.2 P1-12:预算耗尽也算 turn 完成(发 TurnComplete,
                    // 状态 `TokenBudgetExceeded`),区别于 `completed_normally`
                    // (自然结束 / MaxIterations)。两者都走 turn-completion 路径;
                    // 异常取消(error / cancel)仍走 `!completed_normally &&
                    // !budget_exceeded` 的静默丢弃分支。
                    if final_state.completed_normally || final_state.budget_exceeded {
                        // v1.x S4:把 `model.read()` 的 String 快照先 clone 出来,
                        // 否则 RwLockReadGuard 跨 .await → future !Send。
                        // pricing 表 lookup 不需要 RwLock 持有,只要 model 字符串。
                        let model_for_pricing = model.read().clone();
                        let _ = turn_tx_clone
                            .send(Event::new(
                                sub_id_clone.clone(),
                                EventMsg::TokenCount(TokenCountEvent {
                                    input_tokens: final_state.total_usage.input_tokens,
                                    output_tokens: final_state.total_usage.output_tokens,
                                    cached_tokens: final_state.total_usage.cached_tokens,
                                    cache_write_tokens: final_state.total_usage.cache_write_tokens,
                                    total_tokens: final_state.total_usage.total_tokens,
                                    // v1.x S4:用当前 model spec + total_usage 算
                                    // per-turn USD cost;model 不在 pricing 表里
                                    // (未知 / 本地无标价的 model) → None,TUI
                                    // 显示 "$—" 而非 "$0.00"(保守)。
                                    cost_usd: reflect_llm::providers::pricing::price(
                                        &model_for_pricing,
                                        &final_state.total_usage,
                                    ),
                                    ..Default::default()
                                }),
                            ))
                            .await;
                        // v1.2 P1-12:turn 状态优先级 —— 预算耗尽 >
                        // MaxIterations > Success。预算耗尽时 model_call
                        // 提前返回 None,iteration 通常 < 32,故需显式判
                        // budget_exceeded 才不会误报 Success。
                        let status = if final_state.budget_exceeded {
                            TurnStatus::TokenBudgetExceeded
                        } else if final_state.iteration > 32 {
                            TurnStatus::MaxIterations
                        } else {
                            TurnStatus::Success
                        };
                        let _ = turn_tx_clone
                            .send(Event::new(
                                sub_id_clone,
                                EventMsg::TurnComplete(TurnCompleteEvent {
                                    turn_id,
                                    usage: final_state.total_usage,
                                    status,
                                }),
                            ))
                            .await;
                        // M5: persist the assistant's final message so a
                        // later `resume` can replay the conversation history.
                        if let Some(rec) = recorder {
                            let last_assistant_text = final_state
                                .latest_content
                                .iter()
                                .find_map(|b| match b {
                                    reflect_protocol::ContentBlock::Text { text } => {
                                        if text.is_empty() {
                                            None
                                        } else {
                                            Some(text.clone())
                                        }
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();
                            if !last_assistant_text.is_empty() {
                                let _ = rec
                                    .record(RolloutRecord::message(
                                        turn_id,
                                        MessageRole::Assistant,
                                        serde_json::Value::String(last_assistant_text),
                                    ))
                                    .await;
                            }
                        }
                    }
                });
            }
            reflect_protocol::Op::Compact => {
                // v1.2 P1-12(已有-B):`/compact` 设置 `force_compact_next`
                // 标志,让下一个 turn 的 `pre_loop` 强制运行 compactor(无视
                // trigger_tokens 阈值)。架构原因:`Op::Compact` 到达时无活跃
                // turn / 无可压缩 messages,真正的压缩只能在下一个 turn 的
                // `pre_loop`(有 messages 时)做。
                cfg.request_force_compact();
                tracing::info!(
                    "/compact: force_compact_next queued; next turn's pre_loop will compact"
                );
                // emit 一个 Noop event 让 TUI 知道请求已被接受(下一轮
                // pre_loop 才会 emit 真实的 Microcompact/SmartPrune event)。
                let _ = turn_tx
                    .send(Event::new(
                        sub.id,
                        EventMsg::ContextCompacted(ContextCompactedEvent {
                            strategy: ContextCompactedStrategy::Noop,
                            removed_messages: 0,
                            before_tokens: 0,
                            after_tokens: 0,
                        }),
                    ))
                    .await;
            }
            reflect_protocol::Op::Interrupt => {
                let _ = turn_tx
                    .send(Event::new(
                        sub.id,
                        EventMsg::TurnAborted(TurnAbortedEvent {
                            turn_id: TurnId::new(),
                            reason: AbortReason::UserInterrupt,
                        }),
                    ))
                    .await;
            }
            // 批次十九 → 批次二十二:`Op::Rewind` —— 对话回退。
            // 现在真正做持久化截断:调 `RolloutRecorder::truncate_after` 删除
            // 目标 turn(含)及之后的 JSONL 记录(`JsonlRolloutWriter` 会先写
            // `.bak` 备份,可恢复)。引擎的会话内多轮 history 每 turn 重建,
            // 截断后下一次 turn 自然从更短的 rollout 回放 —— 故此处不持有
            // 内存 history 也能正确回退。`None` = 回退到最后一个 turn(最常用)。
            // truncate 失败不致命:warn 后降级为纯事件回执(旧行为)。
            reflect_protocol::Op::Rewind { to_turn_id } => {
                let target = to_turn_id
                    .as_deref()
                    .and_then(|s| reflect_protocol::TurnId::parse_str(s).ok());
                let dropped = match cfg.m4.as_ref().and_then(|m| m.recorder.clone()) {
                    Some(rec) => rec.truncate_after(target.as_ref()).await.unwrap_or_else(|e| {
                        tracing::warn!("rollout truncate_after failed, falling back to event-only rewind: {e:#}");
                        0
                    }),
                    None => 0,
                };
                let _ = turn_tx
                    .send(Event::new(
                        sub.id,
                        EventMsg::TurnRewound(reflect_protocol::TurnRewoundEvent {
                            to_turn_id: to_turn_id.clone(),
                            truncated_after: dropped,
                        }),
                    ))
                    .await;
            }
            reflect_protocol::Op::Shutdown => {
                let ev = Event::new(sub.id, EventMsg::ShutdownComplete);
                let _ = turn_tx.send(ev.clone()).await;
                fan_out_session(&session_subs, &ev);
                break;
            }
            reflect_protocol::Op::ToolApproval { id, decision }
            | reflect_protocol::Op::HookApproval { id, decision } => {
                // M6: hand the user's verdict to the waiting ApprovalGate.
                // Returns false if the waiter has already been removed
                // (timeout / cancel) — we drop silently.
                let resolved = complete_approval(&approval_waiters, &id, decision);
                if !resolved {
                    tracing::debug!(
                        request_id = %id,
                        "approval reply arrived but no waiter; dropping"
                    );
                }
            }
            // ── v1.x Plan mode:完整状态机 ─────────────────────────────
            // 流程:
            // 1. 生成 `PlanId`,在 `plan_approval_gate` 注册 oneshot waiter
            // 2. emit `PlanRequest` / `PlanReady` 等待 TUI 用户审批
            // 3. 阻塞在 `rx.await`;cancel token 触发或 caller 退出 → Deny
            // 4. 决策 approve → flip `PermissionMode` + emit `PermissionModeChanged`
            //    决策 deny → emit `PlanRejected { reason }`
            // 5. `Op::PlanApproval { id, decision }` 由后续轮询的 `match` arm
            //    投递决策(`complete_plan_approval`)唤醒本 waiter
            reflect_protocol::Op::EnterPlanMode { task } => {
                let plan_id = PlanId::new();
                let rx = plan_approval_gate.register(plan_id);
                let _ = turn_tx
                    .send(Event::new(
                        sub.id.clone(),
                        EventMsg::PlanRequest(PlanRequestEvent { task }),
                    ))
                    .await;
                spawn_plan_approval_waiter(
                    rx,
                    plan_id,
                    PermissionMode::Plan,
                    cfg.clone(),
                    sub.id,
                    session_subs.clone(),
                );
            }
            reflect_protocol::Op::ExitPlanMode => {
                let plan_id = PlanId::new();
                let rx = plan_approval_gate.register(plan_id);
                let _ = turn_tx
                    .send(Event::new(
                        sub.id.clone(),
                        EventMsg::PlanReady(PlanReadyEvent {
                            plan_id,
                            markdown: String::from(
                                "(Plan markdown 汇编待 Phase 5 接入;当前 placeholder)",
                            ),
                        }),
                    ))
                    .await;
                spawn_plan_approval_waiter(
                    rx,
                    plan_id,
                    PermissionMode::Prompt,
                    cfg.clone(),
                    sub.id,
                    session_subs.clone(),
                );
            }
            // v1.x Plan mode:`Op::PlanApproval` 由 TUI 在 modal 上按 Y/N/A 后
            // 发出,把决策投递给对应 `PlanId` 的 waiter。`id` 是 String
            // (序列化稳定),parse 到 `PlanId` 才能查到 waiter。
            reflect_protocol::Op::PlanApproval {
                id: plan_id_str,
                decision,
            } => match plan_id_str.parse::<PlanId>() {
                Ok(plan_id) => {
                    let resolved =
                        complete_plan_approval(plan_approval_gate.waiters(), &plan_id, decision);
                    if !resolved {
                        tracing::debug!(plan_id = %plan_id, "plan approval reply arrived but no waiter; dropping");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        plan_id = %plan_id_str,
                        error = %e,
                        "malformed plan_id in Op::PlanApproval; dropping"
                    );
                }
            },
            // v1.x S4:`/effort low|medium|high` slash 的 Op 路径。写
            // `cfg.effort` 槽(下一轮 `model_call` 读最新值构造
            // `ChatRequest::thinking`)。不 emit 任何 Event —— slash 派发
            // 端已直接推 Pill 反馈用户;这里只做审计行 + 写槽。
            reflect_protocol::Op::SetEffort { effort } => {
                tracing::info!(
                    ?effort,
                    "effort override applied; will take effect on next model_call"
                );
                cfg.set_effort(effort);
            }
            // v1.1.0 P1 #14:TUI 在 question modal 上提交 / 取消时,投递结构化
            // 答案回阻塞的 `ask_user_question` 工具。`request_id` 配对
            // `AskUserQuestionEvent.request_id`;`answers` 由 TUI 从
            // `PendingQuestion.selected` + `custom` 序列化得到,用户按 Esc 时
            // 为空 `AskUserAnswer`。
            reflect_protocol::Op::AskUserQuestionResponse { id, answers } => {
                let resolved = complete_ask_user_question(&question_waiters, &id, answers);
                if !resolved {
                    tracing::debug!(
                        request_id = %id,
                        "ask_user_question reply arrived but no waiter; dropping"
                    );
                }
            }
            reflect_protocol::Op::AskUserInputResponse { id, text } => {
                let resolved = complete_ask_user_input(&user_input_waiters, &id, text);
                if !resolved {
                    tracing::debug!(
                        request_id = %id,
                        "ask_user reply arrived but no waiter; dropping"
                    );
                }
            }
            reflect_protocol::Op::SetPermissionMode { mode } => {
                apply_permission_mode_change(&cfg, mode, &session_subs, &turn_subs);
            }
            reflect_protocol::Op::CyclePermissionMode => {
                let next = cfg.permission_mode().next_in_ui_cycle();
                apply_permission_mode_change(&cfg, next, &session_subs, &turn_subs);
            }
        }

        drop(turn_tx);
    }
}

fn provider_of(model_spec: &str) -> String {
    model_spec
        .split_once('/')
        .map(|(p, _)| p.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Fan out a session-level event to every subscriber. Uses `try_send` so a
/// slow / unbounded subscriber never blocks the submission loop; on full or
/// closed channel the event is silently dropped (subscribers see a gap, which
/// is the same semantics as dropping a UI frame).
fn fan_out_session(subs: &Mutex<Vec<mpsc::Sender<Event>>>, ev: &Event) {
    let guard = subs.lock();
    for tx in guard.iter() {
        let _ = tx.try_send(ev.clone());
    }
}

/// v1.1.0 P1:`/mode` / Shift+Tab / `Op::SetPermissionMode` 写入 mode 并广播。
fn apply_permission_mode_change(
    cfg: &AgentConfig,
    to: PermissionMode,
    session_subs: &Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
    turn_subs: &Arc<Mutex<HashMap<String, mpsc::Sender<Event>>>>,
) {
    let from = cfg.permission_mode();
    if from == to {
        return;
    }
    cfg.set_permission_mode(to);
    let ev = Event::new(
        reflect_protocol::EVENT_ID_NONE,
        EventMsg::PermissionModeChanged(PermissionModeChangedEvent { from, to }),
    );
    fan_out_session(session_subs, &ev);
    let guard = turn_subs.lock();
    for tx in guard.values() {
        let _ = tx.try_send(ev.clone());
    }
    tracing::info!(?from, ?to, "permission mode changed");
}

/// v1.x Plan mode:把 plan approval 决策路由到 `PermissionMode` 翻转。
///
/// 由 `Op::EnterPlanMode` / `Op::ExitPlanMode` 触发:
/// - 阻塞在 `rx.await` 等待 `Op::PlanApproval` 的决策
/// - `Approve` / `ApproveForSession` → `cfg.set_permission_mode(target)` +
///   emit `EventMsg::PermissionModeChanged { from, to }`
/// - `Deny { reason }` → emit `EventMsg::PlanRejected { plan_id, reason }`
/// - rx channel 关闭(cancel / caller drop)→ 静默退出,不发事件
///
/// 函数 spawn 到独立 task,因为 `submission_loop` 的主循环要立即回到
/// `sub_rx.recv()` 处理后续 `Op::PlanApproval`,而 plan 决策可能在很久
/// 之后(用户离开键盘几小时)才到。
#[allow(clippy::too_many_arguments)]
fn spawn_plan_approval_waiter(
    rx: tokio::sync::oneshot::Receiver<reflect_protocol::ReviewDecision>,
    plan_id: PlanId,
    target_mode: PermissionMode,
    cfg: AgentConfig,
    sub_id: String,
    session_subs: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
) {
    tokio::spawn(async move {
        let decision = match rx.await {
            Ok(d) => d,
            Err(_) => {
                tracing::debug!(
                    plan_id = %plan_id,
                    "plan approval waiter cancelled (caller dropped or session shutdown)"
                );
                return;
            }
        };
        use reflect_protocol::ReviewDecision as D;
        match decision {
            D::Approve | D::ApproveForSession => {
                let from = cfg.permission_mode();
                if from == target_mode {
                    // 已经处于目标 mode(罕见但可能 —— 重入 EnterPlanMode)。
                    tracing::debug!(
                        plan_id = %plan_id,
                        ?from,
                        "plan approval approved but already in target mode; no-op"
                    );
                    return;
                }
                cfg.set_permission_mode(target_mode);
                let ev = Event::new(
                    sub_id,
                    EventMsg::PermissionModeChanged(PermissionModeChangedEvent {
                        from,
                        to: target_mode,
                    }),
                );
                fan_out_session(&session_subs, &ev);
                tracing::info!(
                    plan_id = %plan_id,
                    ?from,
                    ?target_mode,
                    "plan mode transition committed"
                );
            }
            D::Deny { reason } => {
                tracing::info!(plan_id = %plan_id, %reason, "plan mode transition denied");
                let ev = Event::new(
                    sub_id,
                    EventMsg::PlanRejected(PlanRejectedEvent {
                        plan_id,
                        reason: Some(reason),
                    }),
                );
                fan_out_session(&session_subs, &ev);
            }
        }
    });
}

// Suppress unused import warning for SystemBlocks in builds that don't
// yet wire prompt injection in the StateGraph.
#[allow(dead_code)]
fn _suppress_unused(_s: SystemBlocks) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_of_strips_model_name() {
        assert_eq!(provider_of("openai/gpt-4o"), "openai");
        assert_eq!(provider_of("claude-3"), "unknown");
    }
}
