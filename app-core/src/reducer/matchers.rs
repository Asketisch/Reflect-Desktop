use std::time::SystemTime;

use reflect_protocol::{ApprovalKind as ProtoApprovalKind, EventMsg, TurnId};

use crate::state::{
    ApprovalKind, CollabMessage, CollabSession, CollabStatus, McpInvocation, PendingApproval,
    PendingAskUser, PendingPlan, PendingQuestion, RenderState, TokenUsageSnapshot, Turn,
    TurnStatus,
};

use super::state_mut::{
    map_approval_policy, map_permission_mode, map_sandbox_policy, turn_mut, upsert_server,
};

pub(super) fn apply_session(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::SessionConfigured(e) => {
            state.active_thread = Some(e.session_id);
            state.active_model = e.model;
            state.active_provider = e.provider;
            state.approval_policy = map_approval_policy(&e.approval_policy);
            state.sandbox_policy = map_sandbox_policy(&e.sandbox_policy);
            state.context_window_size = e.context_window_size;
            state.permission_mode = map_permission_mode(&e.approval_policy);
            state.last_error = None;
        }
        EventMsg::TurnStarted(e) => {
            if !state.turns.iter().any(|t| Some(t.id) == turn_id) {
                state.turns.push(Turn {
                    id: e.turn_id,
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
            state.streaming_turn = Some(e.turn_id);
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
            if let Some(target_turn) = e.to_turn_id.and_then(|s| TurnId::parse_str(&s).ok()) {
                state.turns.retain(|t| t.id.0 <= target_turn.0);
            }
        }
        EventMsg::ShutdownComplete => {
            state.shutdown_requested = true;
            state.busy = false;
        }
        _ => unreachable!("session matcher received another event domain"),
    }
}

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
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        _ => unreachable!("approval matcher received another event domain"),
    }
}

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

pub(super) fn apply_context(state: &mut RenderState, msg: EventMsg, turn_id: Option<TurnId>) {
    match msg {
        EventMsg::ContextCompacted(e) => {
            if let Some(t) = turn_mut(state, turn_id) {
                t.error = Some(format!(
                    "[compacted:{} removed={} tokens {}→{}]",
                    serde_json::to_value(&e.strategy)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    e.removed_messages,
                    e.before_tokens,
                    e.after_tokens
                ));
            }
        }
        _ => unreachable!("context matcher received another event domain"),
    }
}

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

pub(super) fn apply_config(state: &mut RenderState, msg: EventMsg) {
    match msg {
        EventMsg::ConfigReloaded(_) => state.config_reloaded_at = Some(SystemTime::now()),
        _ => unreachable!("config matcher received another event domain"),
    }
}

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
            state.pending_plan = Some(PendingPlan {
                id: e.plan_id,
                task: String::new(),
                markdown: Some(e.markdown),
                turn_id: turn_id.unwrap_or_default(),
            });
        }
        EventMsg::PlanApproved(_) | EventMsg::PlanRejected(_) => state.pending_plan = None,
        EventMsg::PermissionModeChanged(e) => state.permission_mode = e.to,
        _ => unreachable!("plan matcher received another event domain"),
    }
}
