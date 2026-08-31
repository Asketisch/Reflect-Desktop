//! Reducer 事件分发器 —— 按 `EventMsg` 领域分组处理，调用对应的状态变更逻辑。
//!
//! 每个 `apply_*` 函数处理一个事件领域（会话、消息、Token、工具、审批、问题、错误等），
//! 内部通过 `match msg` 匹配具体的 `EventMsg` 变体并更新 `RenderState`。

use std::time::SystemTime;

use reflect_protocol::{ApprovalKind as ProtoApprovalKind, EventMsg, TurnId};

use crate::state::{
    ApprovalKind, CollabMessage, CollabSession, CollabStatus, McpInvocation, PendingApproval,
    PendingAskUser, PendingPlan, PendingQuestion, RenderState, TokenUsageSnapshot, Turn,
    TurnStatus,
};

use super::state_mut::{map_approval_policy, map_sandbox_policy, turn_mut, upsert_server};

/// 处理会话级事件：SessionConfigured / TurnStarted / TurnComplete / TurnAborted / TurnRewound / ShutdownComplete。
pub(super) fn apply_session(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::SessionConfigured(e) => {
            state.active_thread = Some(e.session_id);
            state.active_model = e.model;
            state.active_provider = e.provider;
            state.approval_policy = map_approval_policy(&e.approval_policy);
            state.sandbox_policy = map_sandbox_policy(&e.sandbox_policy);
            state.context_window_size = e.context_window_size;
            // 注意:不在这里写 permission_mode。协议端 SessionConfiguredEvent
            // 恒定携带默认 ApprovalPolicy::Auto,若据此覆盖会把 bind 时恢复的
            // Plan/Prompt 等模式冲掉;权限模式的唯一事实源是
            // PermissionModeChanged 事件(与 TS reducer 行为一致)。
            state.last_error = None;
        }
        EventMsg::TurnStarted(e) => {
            // turn 身份键必须与后续 per-turn 事件(AgentMessageDelta /
            // ToolCallBegin / TurnComplete ...)的查找键一致 —— 后端线格式里
            // 事件信封 id = submission id,而 payload 里的 e.turn_id 是另一个
            // 随机 UUID。统一用 event.id 派生的 turn_id(与 TS reducer 对齐),
            // 否则去重失效、所有 per-turn 事件永远挂不上 turn。
            let key = turn_id.unwrap_or(e.turn_id);
            if !state.turns.iter().any(|t| t.id == key) {
                state.turns.push(Turn {
                    id: key,
                    user_text: None,
                    assistant_text: String::new(),
                    thinking: String::new(),
                    tool_calls: Vec::new(),
                    tool_outputs: Vec::new(),
                    error: None,
                    status: TurnStatus::Streaming,
                    started_at: SystemTime::now(),
                });
            }
            state.busy = true;
            state.streaming_turn = Some(key);
        }
        EventMsg::TurnComplete(_) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.status = TurnStatus::Done;
            }
            state.busy = false;
            state.streaming_turn = None;
            state.live_agent_message.clear();
            state.live_thinking.clear();
        }
        EventMsg::TurnAborted(_) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.status = TurnStatus::Aborted;
            }
            state.busy = false;
            state.streaming_turn = None;
        }
        EventMsg::TurnRewound(e) => {
            // TurnId 是随机 v4 UUID,字节序与时间顺序无关,不能比较大小;
            // 按目标 turn 在列表中的索引截断,未知 id 保持不动(后端是
            // 单一事实源,下一次回放会对齐)。与 TS reducer 行为一致。
            if let Some(target_turn) = e.to_turn_id.and_then(|s| TurnId::parse_str(&s).ok()) {
                if let Some(idx) = state.turns.iter().position(|t| t.id == target_turn) {
                    state.turns.truncate(idx + 1);
                }
            }
        }
        EventMsg::ShutdownComplete => {
            state.shutdown_requested = true;
            state.busy = false;
        }
        _ => unreachable!("session matcher received another event domain"),
    }
}

/// 处理 agent 消息事件：AgentMessage / AgentMessageDelta / ThinkingDelta。
pub(super) fn apply_message(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::AgentMessage(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.assistant_text = e.text;
            }
            state.live_agent_message.clear();
        }
        EventMsg::AgentMessageDelta(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.assistant_text.push_str(&e.delta);
            }
            state.live_agent_message.push_str(&e.delta);
        }
        EventMsg::ThinkingDelta(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.thinking.push_str(&e.delta);
            }
            state.live_thinking.push_str(&e.delta);
        }
        _ => unreachable!("message matcher received another event domain"),
    }
}

/// 处理 Token 计数字件：更新 last_token_usage 与 total_cost_usd。
pub(super) fn apply_token(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::TokenCount(e) => {
            state.last_token_usage = Some(TokenUsageSnapshot {
                input: e.input_tokens,
                output: e.output_tokens,
                cached: e.cached_tokens,
                total: e.total_tokens,
                cost_usd: e.cost_usd,
            });
            if let Some(cost) = e.cost_usd {
                state.total_cost_usd += cost;
            }
        }
        _ => unreachable!("token matcher received another event domain"),
    }
}

/// 处理工具调用事件：ToolCallBegin（创建 ToolCall 记录）/ ToolCallEnd（更新状态并追加输出）。
pub(super) fn apply_tool(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::ToolCallBegin(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.tool_calls.push(crate::state::ToolCall {
                    call_id: e.call_id,
                    tool_name: e.tool_name,
                    args: e.args,
                    status: crate::state::ToolStatus::Running,
                });
            }
        }
        EventMsg::ToolCallEnd(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                let status = if e.is_error {
                    crate::state::ToolStatus::Error
                } else {
                    crate::state::ToolStatus::Done
                };
                if let Some(call) = t.tool_calls.iter_mut().find(|c| c.call_id == e.call_id) {
                    call.status = status;
                }
                let text = e
                    .output
                    .content
                    .iter()
                    .filter_map(|cb| match cb {
                        reflect_protocol::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                t.tool_outputs.push(crate::state::ToolOutput {
                    call_id: e.call_id,
                    text,
                    is_error: e.is_error,
                });
            }
        }
        _ => unreachable!("tool matcher received another event domain"),
    }
}

/// 处理审批请求事件：ApprovalRequest / PermissionBubble → 设置 pending_approval。
pub(super) fn apply_approval(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::ApprovalRequest(e) => {
            let proto_kind = e.kind;
            let kind = match &proto_kind {
                ProtoApprovalKind::Tool { .. } => ApprovalKind::Tool,
                ProtoApprovalKind::Hook { .. } => ApprovalKind::Hook,
                ProtoApprovalKind::Plan { .. } => ApprovalKind::Plan,
            };
            let (tool_name, args_summary) = match proto_kind {
                ProtoApprovalKind::Tool { tool_name, args } => (
                    Some(tool_name),
                    Some(serde_json::to_string(&args).unwrap_or_default()),
                ),
                _ => (None, None),
            };
            state.pending_approval = Some(PendingApproval {
                id: e.request_id,
                kind,
                tool_name,
                args_summary,
                risk: None,
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        EventMsg::PermissionBubble(e) => {
            state.pending_approval = Some(PendingApproval {
                id: format!(
                    "bubble-{}-{}",
                    e.tool_name,
                    turn_id.map(|t| t.0.to_string()).unwrap_or_default()
                ),
                kind: ApprovalKind::Tool,
                tool_name: Some(e.tool_name),
                args_summary: e.args_preview,
                risk: Some(e.risk),
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        _ => unreachable!("approval matcher received another event domain"),
    }
}

/// 处理用户选择问题事件：AskUserQuestion → 设置 pending_question。
pub(super) fn apply_question(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::AskUserQuestion(e) => {
            let id = e.request_id.clone();
            state.pending_question = Some(PendingQuestion {
                id,
                event: e,
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        _ => unreachable!("question matcher received another event domain"),
    }
}

/// 处理自由格式输入事件：AskUserInput → 设置 pending_ask_user。
pub(super) fn apply_ask_user(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::AskUserInput(e) => {
            state.pending_ask_user = Some(PendingAskUser {
                id: e.request_id,
                prompt: e.prompt,
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        _ => unreachable!("ask_user matcher received another event domain"),
    }
}

/// 处理上下文压缩事件：ContextCompacted。
pub(super) fn apply_context(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::ContextCompacted(_) => {
            // 压缩是信息性事件(此前被写进 Turn.error,把正常压缩标成错误);
            // RenderState 暂无压缩聚合字段,先不落 turn 状态。
            let _ = (state, turn_id);
        }
        _ => unreachable!("context matcher received another event domain"),
    }
}

/// 处理错误事件：Error / StreamError。会话级错误写入 last_error，Turn 级错误写入 Turn.error。
pub(super) fn apply_error(
    state: &mut RenderState,
    msg: EventMsg,
    turn_id: Option<TurnId>,
    is_session_event: bool,
) {
    let text = match msg {
        EventMsg::Error(e) => format!("{}: {}", e.code, e.message),
        EventMsg::StreamError(e) => format!(
            "stream {}: {} (retry in {}ms)",
            e.code, e.message, e.retry_in_ms
        ),
        _ => unreachable!("error matcher received another event domain"),
    };

    if is_session_event {
        state.last_error = Some(text);
    } else if let Some(t) = turn_mut(state, turn_id) {
        t.error = Some(text);
    }
}

/// 处理配置重载事件：更新 config_reloaded_at 时间戳。
pub(super) fn apply_config(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::ConfigReloaded(_) => state.config_reloaded_at = Some(SystemTime::now()),
        _ => unreachable!("config matcher received another event domain"),
    }
}

/// 处理路由事件：记录最后一次路由切换快照。
pub(super) fn apply_routing(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::Routing(e) => {
            state.last_routing = Some(crate::state::RoutingSnapshot {
                kind: e.kind,
                role: e.role,
                from: e.from_credential,
                to: e.to_credential,
                reason: e.reason,
            });
        }
        _ => unreachable!("routing matcher received another event domain"),
    }
}

/// 处理协作事件：CollabStarted（新建会话）/ CollabMessage（追加消息）/ CollabFinished（标记完成）。
pub(super) fn apply_collab(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::CollabStarted(e) => {
            state.collab_sessions.push(CollabSession {
                id: e.id,
                participants: e.participants,
                mode: e.mode,
                status: CollabStatus::Running,
                outcome: None,
                rounds: None,
                messages: Vec::new(),
            });
        }
        EventMsg::CollabMessage(e) => {
            if let Some(s) = state.collab_sessions.iter_mut().find(|s| s.id == e.id) {
                s.messages.push(CollabMessage {
                    from: e.from,
                    kind: e.kind,
                    content: e.content,
                    round: e.round,
                    at: SystemTime::now(),
                });
            }
        }
        EventMsg::CollabFinished(e) => {
            if let Some(s) = state.collab_sessions.iter_mut().find(|s| s.id == e.id) {
                s.status = CollabStatus::Done;
                s.outcome = Some(e.outcome);
                s.rounds = Some(e.rounds);
            }
        }
        _ => unreachable!("collab matcher received another event domain"),
    }
}

/// 处理 MCP server 事件：Started（upsert 状态）/ Failed（标记失败）/ ToolInvoked（记录调用）。
pub(super) fn apply_mcp(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::McpServerStarted(e) => upsert_server(
            &mut state.mcp_servers,
            &e.server,
            crate::state::ServerStatus::Started,
            Some(format!("{} tools", e.tool_count)),
        ),
        EventMsg::McpServerFailed(e) => upsert_server(
            &mut state.mcp_servers,
            &e.server,
            crate::state::ServerStatus::Failed,
            Some(e.error),
        ),
        EventMsg::McpToolInvoked(e) => {
            state.mcp_invocations.push(McpInvocation {
                server: e.server,
                tool: e.tool,
                call_id: e.call_id,
                at: SystemTime::now(),
            });
            if state.mcp_invocations.len() > 50 {
                let drop = state.mcp_invocations.len() - 50;
                state.mcp_invocations.drain(0..drop);
            }
        }
        _ => unreachable!("mcp matcher received another event domain"),
    }
}

/// 处理 LSP server 事件：Started（upsert 状态）/ Failed（标记失败）。
pub(super) fn apply_lsp(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::LspServerStarted(e) => upsert_server(
            &mut state.lsp_servers,
            &e.server,
            crate::state::ServerStatus::Started,
            Some(format!("{} methods", e.methods.len())),
        ),
        EventMsg::LspServerFailed(e) => upsert_server(
            &mut state.lsp_servers,
            &e.server,
            crate::state::ServerStatus::Failed,
            Some(e.error),
        ),
        _ => unreachable!("lsp matcher received another event domain"),
    }
}

/// 处理 Plan 与权限模式事件：PlanRequest / PlanReady / PlanApproved / PlanRejected / PermissionModeChanged。
pub(super) fn apply_plan(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::PlanRequest(e) => {
            state.pending_plan = Some(PendingPlan {
                id: reflect_protocol::PlanId::new(),
                task: e.task,
                markdown: None,
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        EventMsg::PlanReady(e) => {
            // 保留此前 PlanRequest 事件中的用户可见任务描述,
            // 以便 plan modal 仍能展示 agent 被要求执行的任务。
            let prev_task = state
                .pending_plan
                .as_ref()
                .map(|p| p.task.clone())
                .unwrap_or_default();
            state.pending_plan = Some(PendingPlan {
                id: e.plan_id,
                task: prev_task,
                markdown: Some(e.markdown),
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        EventMsg::PlanApproved(_) | EventMsg::PlanRejected(_) => state.pending_plan = None,
        EventMsg::PermissionModeChanged(e) => state.permission_mode = e.to,
        _ => unreachable!("plan matcher received another event domain"),
    }
}
