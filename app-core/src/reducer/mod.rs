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

mod matchers;
mod state_mut;

use reflect_protocol::{EVENT_ID_NONE, Event, EventMsg, TurnId};

use crate::state::RenderState;

/// Apply a `reflect_protocol::Event` to a `RenderState`, returning a new state.
///
/// `event.id == EVENT_ID_NONE` is treated as a session-level event
/// (lifecycle / no matching submission).
pub fn apply_event(mut state: RenderState, event: Event) -> RenderState {
    let is_session_event = event.id == EVENT_ID_NONE;
    let turn_id = if is_session_event {
        None
    } else {
        TurnId::parse_str(&event.id).ok()
    };

    match event.msg {
        msg @ (EventMsg::SessionConfigured(_)
        | EventMsg::TurnStarted(_)
        | EventMsg::TurnComplete(_)
        | EventMsg::TurnAborted(_)
        | EventMsg::TurnRewound(_)
        | EventMsg::ShutdownComplete) => matchers::apply_session(&mut state, msg, turn_id),

        msg @ (EventMsg::AgentMessage(_)
        | EventMsg::AgentMessageDelta(_)
        | EventMsg::ThinkingDelta(_)) => matchers::apply_message(&mut state, msg, turn_id),

        msg @ EventMsg::TokenCount(_) => matchers::apply_token(&mut state, msg),

        msg @ (EventMsg::ToolCallBegin(_) | EventMsg::ToolCallEnd(_)) => {
            matchers::apply_tool(&mut state, msg, turn_id)
        }

        msg @ (EventMsg::ApprovalRequest(_) | EventMsg::PermissionBubble(_)) => {
            matchers::apply_approval(&mut state, msg, turn_id)
        }

        msg @ EventMsg::AskUserQuestion(_) => matchers::apply_question(&mut state, msg, turn_id),

        msg @ EventMsg::AskUserInput(_) => matchers::apply_ask_user(&mut state, msg, turn_id),

        msg @ EventMsg::ContextCompacted(_) => matchers::apply_context(&mut state, msg, turn_id),

        msg @ (EventMsg::Error(_) | EventMsg::StreamError(_)) => {
            matchers::apply_error(&mut state, msg, turn_id, is_session_event)
        }

        msg @ EventMsg::ConfigReloaded(_) => matchers::apply_config(&mut state, msg),
        msg @ EventMsg::Routing(_) => matchers::apply_routing(&mut state, msg),

        msg @ (EventMsg::CollabStarted(_)
        | EventMsg::CollabMessage(_)
        | EventMsg::CollabFinished(_)) => matchers::apply_collab(&mut state, msg),

        msg @ (EventMsg::McpServerStarted(_)
        | EventMsg::McpServerFailed(_)
        | EventMsg::McpToolInvoked(_)) => matchers::apply_mcp(&mut state, msg),

        msg @ (EventMsg::LspServerStarted(_) | EventMsg::LspServerFailed(_)) => {
            matchers::apply_lsp(&mut state, msg)
        }

        msg @ (EventMsg::PlanRequest(_)
        | EventMsg::PlanReady(_)
        | EventMsg::PlanApproved(_)
        | EventMsg::PlanRejected(_)
        | EventMsg::PermissionModeChanged(_)) => matchers::apply_plan(&mut state, msg, turn_id),

        // 协议层新增的 event variant,Desktop reducer 暂无专门处理逻辑;
        // 显式列出并 no-op,避免 match 非穷尽编译错误,后续按需接入渲染。
        EventMsg::PlanStep(_) | EventMsg::PluginLoaded(_) | EventMsg::QuotaExhausted(_) => {}
    }

    state
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
    use reflect_protocol::{
        AgentMessage, AgentMessageDelta, SessionConfiguredEvent, TokenCountEvent,
    };
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
                EventMsg::AgentMessageDelta(AgentMessageDelta {
                    delta: "Hello".into(),
                }),
            ),
        );
        state = apply_event(
            state,
            ev(
                &tid.0.to_string(),
                EventMsg::AgentMessageDelta(AgentMessageDelta {
                    delta: " world".into(),
                }),
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
                EventMsg::AgentMessage(AgentMessage {
                    text: "final".into(),
                }),
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
        assert_eq!(state.turns[0].status, crate::state::TurnStatus::Done);
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
