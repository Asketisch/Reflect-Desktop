//! `reflect_app_core::reducer` —— Shared reducer (B2-02).
//!
//! Pure function `reduce_event(state, event) -> state` consumed by both
//! TUI and GUI. Mirrors the TS `reduceEvent` in `src/stores/agentStore.ts`.
//! Returns a new `RenderState` (immutable; `RenderState` derives `Clone`).
//!
//! Design:
//! - No IO, no async, no time-of-day side effects (`SystemTime::now()` is
//!   an exception; the `at` fields on collab messages and config reload
//!   use it for diagnostic ordering. Tests can pass a fixed time.)
//! - `state.apply_event(event)` is the convenience method; the free
//!   function `apply_event(state, event)` is also exported for external
//!   consumers (e.g. NAPI binding).
//! - All 33 `EventMsg` variants are handled.

use std::time::SystemTime;

use reflect_protocol::{ApprovalKind as ProtoApprovalKind, Event, EventMsg, TurnId};

use crate::state::{
    ApprovalKind, CollabMessage, CollabSession, CollabStatus, McpInvocation, PendingApproval,
    PendingAskUser, PendingPlan, PendingQuestion, RenderState, ServerState, ServerStatus,
    TokenUsageSnapshot, Turn, TurnStatus,
};

/// Apply a `reflect_protocol::Event` to a `RenderState`, returning a new state.
///
/// `event.id == EVENT_ID_NONE` is treated as a session-level event
/// (lifecycle / no matching submission).
pub fn apply_event(mut state: RenderState, event: Event) -> RenderState {
    let turn_id_parsed: Option<TurnId> = if event.id.is_empty() {
        None
    } else {
        TurnId::parse_str(&event.id).ok()
    };

    match event.msg {
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
            // Create a new empty turn unless one already exists.
            if !state.turns.iter().any(|t| Some(t.id) == turn_id_parsed) {
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

        EventMsg::TurnComplete(_e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.status = TurnStatus::Done;
                }
            }
            state.busy = false;
            state.streaming_turn = None;
            state.live_agent_message.clear();
            state.live_thinking.clear();
        }

        EventMsg::TurnAborted(_e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.status = TurnStatus::Aborted;
                }
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

        EventMsg::AgentMessage(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.assistant_text = e.text;
                }
            }
            state.live_agent_message.clear();
        }

        EventMsg::AgentMessageDelta(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.assistant_text.push_str(&e.delta);
                }
            }
            state.live_agent_message.push_str(&e.delta);
        }

        EventMsg::ThinkingDelta(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.thinking.push_str(&e.delta);
                }
            }
            state.live_thinking.push_str(&e.delta);
        }

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

        EventMsg::ToolCallBegin(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.tool_calls.push(crate::state::ToolCall {
                        call_id: e.call_id,
                        tool_name: e.tool_name,
                        args: e.args,
                        status: crate::state::ToolStatus::Running,
                    });
                }
            }
        }

        EventMsg::ToolCallEnd(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
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
        }

        EventMsg::ApprovalRequest(e) => {
            let proto_kind = e.kind;
            let kind = match &proto_kind {
                ProtoApprovalKind::Tool { .. } => ApprovalKind::Tool,
                ProtoApprovalKind::Hook { .. } => ApprovalKind::Hook,
                ProtoApprovalKind::Plan { .. } => ApprovalKind::Plan,
            };
            let (tool_name, args_summary) = match proto_kind {
                ProtoApprovalKind::Tool { tool_name, args } => {
                    (Some(tool_name), Some(serde_json::to_string(&args).unwrap_or_default()))
                }
                _ => (None, None),
            };
            state.pending_approval = Some(PendingApproval {
                id: e.request_id,
                kind,
                tool_name,
                args_summary,
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::AskUserQuestion(e) => {
            let id = e.request_id.clone();
            state.pending_question = Some(PendingQuestion {
                id,
                event: e,
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::AskUserInput(e) => {
            state.pending_ask_user = Some(PendingAskUser {
                id: e.request_id,
                prompt: e.prompt,
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::PermissionBubble(e) => {
            state.pending_approval = Some(PendingApproval {
                id: format!("bubble-{}-{}", e.tool_name, turn_id_parsed.map(|t| t.0.to_string()).unwrap_or_default()),
                kind: ApprovalKind::Tool,
                tool_name: Some(e.tool_name),
                args_summary: e.args_preview,
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::ContextCompacted(e) => {
            if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
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
        }

        EventMsg::Error(e) => {
            let text = format!("{}: {}", e.code, e.message);
            if event.id.is_empty() {
                state.last_error = Some(text);
            } else if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.error = Some(text);
                }
            }
        }

        EventMsg::StreamError(e) => {
            let text = format!("stream {}: {} (retry in {}ms)", e.code, e.message, e.retry_in_ms);
            if event.id.is_empty() {
                state.last_error = Some(text);
            } else if let Some(tid) = turn_id_parsed {
                if let Some(t) = state.turns.iter_mut().find(|t| Some(t.id) == Some(tid)) {
                    t.error = Some(text);
                }
            }
        }

        EventMsg::ConfigReloaded(_e) => {
            state.config_reloaded_at = Some(SystemTime::now());
        }

        EventMsg::Routing(e) => {
            state.last_routing = Some(crate::state::RoutingSnapshot {
                kind: e.kind,
                role: e.role,
                from: e.from_credential,
                to: e.to_credential,
                reason: e.reason,
            });
        }

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

        EventMsg::McpServerStarted(e) => {
            upsert_server(&mut state.mcp_servers, &e.server, ServerStatus::Started, Some(format!("{} tools", e.tool_count)));
        }

        EventMsg::McpServerFailed(e) => {
            upsert_server(&mut state.mcp_servers, &e.server, ServerStatus::Failed, Some(e.error));
        }

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

        EventMsg::LspServerStarted(e) => {
            upsert_server(&mut state.lsp_servers, &e.server, ServerStatus::Started, Some(format!("{} methods", e.methods.len())));
        }

        EventMsg::LspServerFailed(e) => {
            upsert_server(&mut state.lsp_servers, &e.server, ServerStatus::Failed, Some(e.error));
        }

        EventMsg::PlanRequest(e) => {
            state.pending_plan = Some(PendingPlan {
                id: reflect_protocol::PlanId::new(),
                task: e.task,
                markdown: None,
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::PlanReady(e) => {
            state.pending_plan = Some(PendingPlan {
                id: e.plan_id,
                task: String::new(),
                markdown: Some(e.markdown),
                turn_id: turn_id_parsed.unwrap_or(TurnId::default()),
            });
        }

        EventMsg::PlanApproved(_) | EventMsg::PlanRejected(_) => {
            state.pending_plan = None;
        }

        EventMsg::PermissionModeChanged(e) => {
            state.permission_mode = e.to;
        }
    }

    state
}

fn upsert_server(
    list: &mut Vec<ServerState>,
    name: &str,
    status: ServerStatus,
    detail: Option<String>,
) {
    if let Some(s) = list.iter_mut().find(|s| s.name == name) {
        s.status = status;
        s.detail = detail;
    } else {
        list.push(ServerState {
            name: name.to_string(),
            status,
            detail,
        });
    }
}

fn map_approval_policy(_p: &reflect_protocol::ApprovalPolicy) -> crate::state::ApprovalPolicy {
    // reflect-protocol uses stringly-typed policy; for now, default to Auto.
    // Real mapping can read a field if needed.
    crate::state::ApprovalPolicy::Auto
}

fn map_sandbox_policy(_p: &reflect_protocol::SandboxPolicy) -> crate::state::SandboxPolicy {
    crate::state::SandboxPolicy::WorkspaceOnly
}

fn map_permission_mode(p: &reflect_protocol::ApprovalPolicy) -> reflect_protocol::PermissionMode {
    // ApprovalPolicy is snake_case stringly; PermissionMode has more variants.
    // For Phase 1, return Auto; future work reads the actual value.
    let _ = p;
    reflect_protocol::PermissionMode::Auto
}

impl RenderState {
    /// Apply an event to `self`, returning a new `RenderState`.
    pub fn apply_event(&self, event: Event) -> Self {
        apply_event(self.clone(), event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::{AgentMessage, AgentMessageDelta, SessionConfiguredEvent, TokenCountEvent};
    use std::time::{Duration, UNIX_EPOCH};

    fn ev(id: &str, msg: EventMsg) -> Event {
        Event {
            id: id.to_string(),
            msg,
        }
    }

    #[test]
    fn session_configured_sets_active_thread_and_model() {
        let mut state = RenderState::default();
        let sc = SessionConfiguredEvent::new("anthropic/claude-opus-4-7", "anthropic");
        let _id = sc.session_id;
        state = apply_event(state, ev("", EventMsg::SessionConfigured(sc.clone())));
        assert_eq!(state.active_model, "anthropic/claude-opus-4-7");
        assert_eq!(state.active_provider, "anthropic");
    }

    #[test]
    fn turn_started_creates_empty_turn() {
        let state = RenderState::default();
        let tid = TurnId::new();
        let state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::TurnStarted(reflect_protocol::TurnStartedEvent {
                    turn_id: tid,
                    user_message_id: None,
                }),
            ),
        );
        assert_eq!(state.turns.len(), 1);
        assert!(state.busy);
    }

    #[test]
    fn agent_message_delta_appends_to_live_and_turn() {
        let mut state = RenderState::default();
        let tid = TurnId::new();
        state = apply_event(
            state.clone(),
            ev(
                &tid.0.to_string(),
                EventMsg::TurnStarted(reflect_protocol::TurnStartedEvent {
                    turn_id: tid,
                    user_message_id: None,
                }),
            ),
        );
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::AgentMessageDelta(AgentMessageDelta { delta: "Hello".into() }),
            ),
        );
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::AgentMessageDelta(AgentMessageDelta { delta: " world".into() }),
            ),
        );
        assert_eq!(state.turns[0].assistant_text, "Hello world");
        assert_eq!(state.live_agent_message, "Hello world");
    }

    #[test]
    fn agent_message_finalizes_assistant_text() {
        let mut state = RenderState::default();
        let tid = TurnId::new();
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::TurnStarted(reflect_protocol::TurnStartedEvent {
                    turn_id: tid,
                    user_message_id: None,
                }),
            ),
        );
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::AgentMessage(AgentMessage { text: "final".into() }),
            ),
        );
        assert_eq!(state.turns[0].assistant_text, "final");
        assert_eq!(state.live_agent_message, "");
    }

    #[test]
    fn token_count_updates_snapshot_and_total_cost() {
        let state = RenderState::default();
        let state = apply_event(
            state,
            ev(
                "",
                EventMsg::TokenCount(TokenCountEvent {
                    input_tokens: 100,
                    output_tokens: 50,
                    cached_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: 150,
                    cost_usd: Some(0.01),
                    ..Default::default()
                }),
            ),
        );
        let snap = state.last_token_usage.unwrap();
        assert_eq!(snap.input, 100);
        assert!((state.total_cost_usd - 0.01).abs() < 1e-9);
    }

    #[test]
    fn turn_complete_marks_done() {
        let mut state = RenderState::default();
        let tid = TurnId::new();
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::TurnStarted(reflect_protocol::TurnStartedEvent {
                    turn_id: tid,
                    user_message_id: None,
                }),
            ),
        );
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::TurnComplete(reflect_protocol::TurnCompleteEvent {
                    turn_id: tid,
                    usage: Default::default(),
                    status: reflect_protocol::TurnStatus::Success,
                }),
            ),
        );
        assert!(!state.busy);
        assert_eq!(state.turns[0].status, TurnStatus::Done);
    }

    #[test]
    fn session_event_does_not_attach_to_turn() {
        let state = RenderState::default();
        let state = apply_event(
            state,
            ev(
                "",
                EventMsg::Error(reflect_protocol::ErrorEvent {
                    code: "E_X".into(),
                    message: "boom".into(),
                    details: None,
                }),
            ),
        );
        assert!(state.turns.is_empty());
        assert!(state.last_error.unwrap().contains("boom"));
    }

    #[test]
    fn turn_id_helper() {
        let _ = TurnId::default();
        let _ = UNIX_EPOCH + Duration::from_secs(0);
    }
}
