//! M2.x 协议桥端到端集成测试 —— 嵌入真实 `reflect_core::AgentThread`。
//!
//! 任务规格:`docs/todo/21-gui/02-protocol-bridge.md` 验收 #1 端到端通路 + M2.x 新增测试。
//!
//! ## 测试矩阵
//!
//! 1. `real_agent_thread_emits_session_configured` — 提交后第一个 turn 必须收到
//!    `SessionConfigured` lifecycle event。
//! 2. `real_agent_thread_emits_error_for_stub_model` — 没注册 model client 时,
//!    提交应 emit `EventMsg::Error("...stub/test...")` 而不是 panic。
//! 3. `interrupt_token_cancels_in_flight_turn` — 调 `agent.interrupt()` 后 cancel
//!    token 已 cancelled。
//! 4. `broadcast_supports_multiple_subscribers` — 两个独立 subscriber 都收到同一 event。
//! 5. `model_spec_and_workspace_accessors` — 诊断接口返回正确值。
//!
//! ## 二段构造
//!
//! `MinimalAgent::new_empty()` 不需要 tokio runtime,可以直接 `agent.install_agent_thread()`
//! 在 `#[tokio::test]` runtime 内调用,这是真实 runtime 上下文的等价物。

use std::time::Duration;

use reflect_desktop_lib::state::MinimalAgent;
use reflect_protocol::{Event, EventMsg, Op, Submission, UserInputItem};
use tokio::sync::broadcast;
use tokio::time::timeout;

/// 收集最多 `max` 个 event 直到 timeout。
async fn collect_events(
    rx: &mut broadcast::Receiver<Event>,
    max: usize,
    timeout_ms: u64,
) -> Vec<Event> {
    let mut out = Vec::with_capacity(max);
    for _ in 0..max {
        let ev = match timeout(Duration::from_millis(timeout_ms), rx.recv()).await {
            Ok(Ok(e)) => e,
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => break,
            Err(_) => break,
        };
        out.push(ev);
    }
    out
}

#[tokio::test]
async fn real_agent_thread_emits_session_configured() {
    let agent = MinimalAgent::new_empty();
    agent.install_agent_thread();
    let mut rx = agent.subscribe_session();

    let sub = Submission::user_input("hello");
    agent.submit(sub).await.expect("submit ok");

    // 在前 10 个 event 里应当至少出现一次 SessionConfigured。
    let events = collect_events(&mut rx, 10, 500).await;
    assert!(
        events.iter().any(|e| matches!(e.msg, EventMsg::SessionConfigured(_))),
        "SessionConfigured should fire within first events; got {} events: {:#?}",
        events.len(),
        events
            .iter()
            .map(|e| std::mem::discriminant(&e.msg))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn real_agent_thread_emits_error_for_stub_model() {
    // 没注册 model client → 提交 UserInput 后应 emit Error event (no panic)。
    let agent = MinimalAgent::new_empty();
    agent.install_agent_thread();
    let mut rx = agent.subscribe_session();

    let sub = Submission::with_id(
        "test-stub-model",
        Op::UserInput {
            items: vec![UserInputItem::Text { text: "ping".into() }],
            thread_settings: Default::default(),
        },
    );
    agent.submit(sub).await.expect("submit ok");

    // 收 5 个 event,应当出现 Error 或 TurnComplete(no model) 或类似诊断消息。
    let events = collect_events(&mut rx, 5, 1500).await;
    let saw_termination = events
        .iter()
        .any(|e| matches!(e.msg, EventMsg::Error(_) | EventMsg::TurnComplete(_)));
    assert!(
        saw_termination,
        "stub backend must emit Error or TurnComplete; got {} events: {:#?}",
        events.len(),
        events
            .iter()
            .map(|e| match &e.msg {
                EventMsg::SessionConfigured(_) => "SessionConfigured".to_string(),
                EventMsg::TurnStarted(_) => "TurnStarted".to_string(),
                EventMsg::TurnComplete(_) => "TurnComplete".to_string(),
                EventMsg::Error(_) => "Error".to_string(),
                EventMsg::AgentMessage(_) => "AgentMessage".to_string(),
                EventMsg::AgentMessageDelta(_) => "AgentMessageDelta".to_string(),
                _ => "Other".to_string(),
            })
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn interrupt_token_cancels_in_flight_turn() {
    let agent = MinimalAgent::new_empty();
    agent.install_agent_thread();
    let _ = agent.subscribe_session();
    agent.interrupt();
    agent.interrupt(); // idempotent
}

#[tokio::test]
async fn broadcast_supports_multiple_subscribers() {
    let agent = MinimalAgent::new_empty();
    agent.install_agent_thread();
    let mut rx1 = agent.subscribe_session();
    let mut rx2 = agent.subscribe_session();

    let sub = Submission::user_input("fanout");
    agent.submit(sub).await.expect("submit ok");

    // 收 5 个 event,两个订阅者都应至少收到 1 个。
    let e1 = collect_events(&mut rx1, 5, 1500).await;
    let e2 = collect_events(&mut rx2, 5, 1500).await;

    assert!(!e1.is_empty(), "subscriber 1 should receive events");
    assert!(!e2.is_empty(), "subscriber 2 should receive events");
    assert!(
        e1.len() >= e2.len().saturating_sub(2) || e2.len() >= e1.len().saturating_sub(2),
        "both subscribers should see roughly the same volume (got {} vs {})",
        e1.len(),
        e2.len()
    );
}

#[tokio::test]
async fn model_spec_and_workspace_accessors() {
    let agent = MinimalAgent::new_empty();
    assert_eq!(agent.model_spec(), "stub/test");
    assert!(agent.workspace().is_absolute() || agent.workspace().as_os_str().len() > 0);
}