//! End-to-end tests for `AgentThread` using a stub `ModelClient`.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt, stream};
use parking_lot::Mutex;
use reflect_core::{AgentConfig, AgentThread};
use reflect_llm::{
    Capabilities, ChatEvent, ChatRequest, CredentialPool, LlmError, ModelClient, ModelRegistry,
    PoolEntry,
};
use reflect_protocol::{EventMsg, RoutingEventKind, Submission, UserInputItem};
use reflect_tools::{ToolRegistry, builtins::EchoTool};
use tokio_util::sync::CancellationToken;

struct StubClient {
    events: Mutex<Vec<ChatEvent>>,
    /// If set, stream returns this error from `stream()` (synchronous failure).
    sync_error: Mutex<Option<LlmError>>,
    /// If set, each yielded Ok event is followed by this error mid-stream.
    mid_stream_error: Mutex<Option<LlmError>>,
    /// How many times `stream` was called.
    call_count: Mutex<u32>,
}

impl StubClient {
    fn new(events: Vec<ChatEvent>) -> Self {
        Self {
            events: Mutex::new(events),
            sync_error: Mutex::new(None),
            mid_stream_error: Mutex::new(None),
            call_count: Mutex::new(0),
        }
    }

    fn with_sync_error(err: LlmError) -> Self {
        let s = Self::new(vec![]);
        *s.sync_error.lock() = Some(err);
        s
    }

    #[allow(dead_code)]
    fn with_mid_stream_error(events: Vec<ChatEvent>, err: LlmError) -> Self {
        let s = Self::new(events);
        *s.mid_stream_error.lock() = Some(err);
        s
    }
}

#[async_trait]
impl ModelClient for StubClient {
    fn name(&self) -> &str {
        "stub"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    async fn stream(
        &self,
        _request: ChatRequest,
        _cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError> {
        *self.call_count.lock() += 1;
        if let Some(e) = self.sync_error.lock().clone() {
            return Err(e);
        }
        let events = self.events.lock().clone();
        let mid = self.mid_stream_error.lock().clone();
        let s: Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>> =
            if let Some(e) = mid {
                let head = stream::iter(events.into_iter().map(Ok));
                let tail = stream::iter(std::iter::once(Err(e)));
                Box::pin(head.chain(tail))
            } else {
                Box::pin(stream::iter(events.into_iter().map(Ok)))
            };
        Ok(s)
    }
}

fn build_thread(registry: Arc<ModelRegistry>) -> AgentThread {
    let cfg = AgentConfig::new("stub/m1", Path::new("."));
    let tools = Arc::new(ToolRegistry::default());
    tools.register(Arc::new(EchoTool));
    AgentThread::new(cfg, registry, tools, None)
}

fn make_sub(text: &str) -> Submission {
    Submission {
        id: "test-sub".into(),
        op: reflect_protocol::Op::UserInput {
            items: vec![UserInputItem::Text { text: text.into() }],
            thread_settings: Default::default(),
        },
        client_user_message_id: None,
        trace: None,
    }
}

#[tokio::test]
async fn single_turn_emits_expected_events() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("hi".into()),
        ChatEvent::Usage {
            input_tokens: 5,
            output_tokens: 1,
            cached_tokens: 0,
            cache_write_tokens: 0,
        },
        ChatEvent::MessageStop,
    ]));
    registry.register_pool(
        "stub",
        CredentialPool {
            entries: vec![PoolEntry {
                client: stub.clone(),
                label: "default".into(),
                weight: 1,
            }],
        },
    );
    let thread = build_thread(registry);

    let mut handle = thread.submit(make_sub("echo hi")).await;

    let mut got: Vec<EventMsg> = Vec::new();
    while let Some(ev) = handle.next().await {
        got.push(ev.msg.clone());
    }

    // Sequence: SessionConfigured, TurnStarted, AgentMessageDelta("hi"),
    // TokenCount, TurnComplete
    assert!(matches!(got[0], EventMsg::SessionConfigured(_)));
    assert!(matches!(got[1], EventMsg::TurnStarted(_)));
    match &got[2] {
        EventMsg::AgentMessageDelta(d) => assert_eq!(d.delta, "hi"),
        other => panic!("expected AgentMessageDelta, got {other:?}"),
    }
    assert!(matches!(got[3], EventMsg::TokenCount(_)));
    let last = got.last().unwrap();
    assert!(matches!(last, EventMsg::TurnComplete(_)));
}

#[tokio::test]
async fn handles_tool_call_gracefully() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ToolUseStart {
            id: "tc1".into(),
            name: "bash".into(),
            input_json: String::new(),
        },
        ChatEvent::ToolUseDelta("{\"cmd\":\"ls\"}".into()),
        ChatEvent::MessageStop,
    ]));
    registry.register_pool(
        "stub",
        CredentialPool {
            entries: vec![PoolEntry {
                client: stub.clone(),
                label: "default".into(),
                weight: 1,
            }],
        },
    );
    let thread = build_thread(registry);
    let mut handle = thread.submit(make_sub("list")).await;

    let mut got: Vec<EventMsg> = Vec::new();
    while let Some(ev) = handle.next().await {
        got.push(ev.msg.clone());
    }
    let saw_begin = got
        .iter()
        .any(|m| matches!(m, EventMsg::ToolCallBegin(e) if e.call_id == "tc1"));
    let saw_end = got
        .iter()
        .any(|m| matches!(m, EventMsg::ToolCallEnd(e) if e.is_error));
    assert!(saw_begin, "expected ToolCallBegin");
    assert!(saw_end, "expected ToolCallEnd with is_error=true");
    // Must still produce a TurnComplete.
    assert!(got.iter().any(|m| matches!(m, EventMsg::TurnComplete(_))));
}

#[tokio::test]
async fn propagates_auth_error_without_turn_complete() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::with_sync_error(LlmError::Auth));
    registry.register_pool(
        "stub",
        CredentialPool {
            entries: vec![PoolEntry {
                client: stub.clone(),
                label: "default".into(),
                weight: 1,
            }],
        },
    );
    let thread = build_thread(registry);
    let mut handle = thread.submit(make_sub("hi")).await;

    let mut saw_error = false;
    let mut saw_turn_complete = false;
    while let Some(ev) = handle.next().await {
        match ev.msg {
            // v1.0 多 Provider 路由:Auth 走 CooldownAndFailover,单
            // credential 池时最终报 `ALL_CREDENTIALS_EXHAUSTED`,
            // details.tried 含原 AUTH_FAILED outcome。
            EventMsg::Error(ref e) if e.code == "ALL_CREDENTIALS_EXHAUSTED" => {
                saw_error = true;
                if let Some(details) = &e.details {
                    let tried = details.get("tried").and_then(|v| v.as_array());
                    if let Some(arr) = tried {
                        assert!(
                            arr.iter()
                                .any(|c| c.get("outcome").and_then(|o| o.as_str()) == Some("auth")),
                            "expected an auth outcome in tried: {details}"
                        );
                    }
                }
            }
            EventMsg::TurnComplete(_) => saw_turn_complete = true,
            _ => {}
        }
    }
    assert!(saw_error, "expected ALL_CREDENTIALS_EXHAUSTED error event");
    assert!(
        !saw_turn_complete,
        "auth error should not produce TurnComplete"
    );
}

#[tokio::test]
async fn retries_on_rate_limit_then_succeeds() {
    let registry = Arc::new(ModelRegistry::new());
    // v1.0 多 Provider 路由:RateLimited 触发 CooldownAndFailover,
    // 所以测试场景是"两个 credential 共享同一 pool,第一个 rate
    // -limited 失败,第二个成功" —— 旧测试"同 credential 重试"在
    // 新语义下已不再适用。
    let call_count_1 = Arc::new(Mutex::new(0u32));
    let call_count_2 = Arc::new(Mutex::new(0u32));
    let call_count_1_c = call_count_1.clone();
    let call_count_2_c = call_count_2.clone();
    struct FlakyClient {
        #[allow(dead_code)] // 留作诊断,registry 不读
        label: &'static str,
        call_count: Arc<Mutex<u32>>,
        first_error: Option<LlmError>,
    }
    #[async_trait]
    impl ModelClient for FlakyClient {
        fn name(&self) -> &str {
            "flaky"
        }
        async fn stream(
            &self,
            _req: ChatRequest,
            _cancel: CancellationToken,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError>
        {
            let mut n = self.call_count.lock();
            *n += 1;
            if *n == 1
                && let Some(err) = self.first_error.clone()
            {
                return Err(err);
            }
            Ok(Box::pin(stream::iter(vec![
                Ok(ChatEvent::ContentDelta("ok".into())),
                Ok(ChatEvent::MessageStop),
            ])))
        }
    }
    let c1: Arc<dyn ModelClient> = Arc::new(FlakyClient {
        label: "work",
        call_count: call_count_1_c,
        first_error: Some(LlmError::RateLimited { retry_after_ms: 5 }),
    });
    let c2: Arc<dyn ModelClient> = Arc::new(FlakyClient {
        label: "personal",
        call_count: call_count_2_c,
        first_error: None,
    });
    registry.register_pool(
        "flaky",
        CredentialPool {
            entries: vec![
                PoolEntry {
                    client: c1,
                    label: "work".into(),
                    weight: 1,
                },
                PoolEntry {
                    client: c2,
                    label: "personal".into(),
                    weight: 1,
                },
            ],
        },
    );
    let cfg = AgentConfig::new("flaky/x", Path::new("."));
    let tools = Arc::new(ToolRegistry::default());
    let thread = AgentThread::new(cfg, registry, tools, None);
    let mut handle = thread.submit(make_sub("retry me")).await;

    let mut saw_stream_error = false;
    let mut saw_routing_switched = false;
    let mut saw_completion = false;
    while let Some(ev) = handle.next().await {
        match ev.msg {
            EventMsg::StreamError(ref s) if s.code == "RATE_LIMITED" => {
                saw_stream_error = true;
            }
            EventMsg::Routing(ref r)
                if matches!(r.kind, RoutingEventKind::Switched) && r.role == "main" =>
            {
                saw_routing_switched = true;
            }
            EventMsg::TurnComplete(_) => saw_completion = true,
            _ => {}
        }
    }
    assert!(saw_stream_error, "expected StreamError on rate limit");
    assert!(
        saw_routing_switched,
        "expected RoutingEvent(Switched) on credential failover"
    );
    assert!(saw_completion, "expected successful completion after retry");
    assert_eq!(*call_count_1.lock(), 1, "work credential called once");
    assert_eq!(
        *call_count_2.lock(),
        1,
        "personal credential called once after failover"
    );
}

#[tokio::test]
async fn multi_turn_emits_session_configured_only_once() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("hi".into()),
        ChatEvent::MessageStop,
    ]));
    registry.register_pool(
        "stub",
        CredentialPool {
            entries: vec![PoolEntry {
                client: stub.clone(),
                label: "default".into(),
                weight: 1,
            }],
        },
    );
    let thread = build_thread(registry);

    // First turn.
    let mut h1 = thread.submit(make_sub("first")).await;
    let mut count1 = 0;
    while let Some(ev) = h1.next().await {
        if matches!(ev.msg, EventMsg::SessionConfigured(_)) {
            count1 += 1;
        }
    }
    assert_eq!(
        count1, 1,
        "SessionConfigured should be emitted exactly once on first turn"
    );

    // Second turn — SessionConfigured should NOT be re-emitted.
    let mut h2 = thread.submit(make_sub("second")).await;
    let mut count2 = 0;
    while let Some(ev) = h2.next().await {
        if matches!(ev.msg, EventMsg::SessionConfigured(_)) {
            count2 += 1;
        }
    }
    assert_eq!(
        count2, 0,
        "SessionConfigured should not re-emit on second turn"
    );
    assert!(h2.next().await.is_none(), "channel should close after turn");
}

#[tokio::test]
async fn cancellation_mid_stream_emits_turn_aborted() {
    use std::time::Duration;
    let registry = Arc::new(ModelRegistry::new());

    // A stub whose stream holds open until cancelled.
    struct SlowClient;
    #[async_trait]
    impl ModelClient for SlowClient {
        fn name(&self) -> &str {
            "slow"
        }
        async fn stream(
            &self,
            _req: ChatRequest,
            cancel: CancellationToken,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError>
        {
            // Emit MessageStart then sleep until cancelled.
            let s = async_stream::stream! {
                yield Ok(ChatEvent::MessageStart { id: "m".into(), model: "slow-1".into() });
                tokio::time::sleep(Duration::from_millis(500)).await;
                yield Ok(ChatEvent::MessageStop);
            };
            // Race the stream against the cancel token: if cancel fires,
            // surface Cancelled.
            let cancel = cancel.clone();
            let s = async_stream::stream! {
                let mut s = std::pin::pin!(s);
                loop {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            yield Ok(ChatEvent::Error(LlmError::Cancelled));
                            return;
                        }
                        evt = s.next() => {
                            match evt {
                                Some(e) => yield e,
                                None => return,
                            }
                        }
                    }
                }
            };
            Ok(Box::pin(s))
        }
    }
    registry.register_pool(
        "slow",
        CredentialPool {
            entries: vec![PoolEntry {
                client: Arc::new(SlowClient),
                label: "default".into(),
                weight: 1,
            }],
        },
    );

    let cancel = CancellationToken::new();
    let cfg = AgentConfig::new("slow/x", Path::new(".")).with_cancel(cancel.clone());
    let tools = Arc::new(ToolRegistry::default());
    let thread = AgentThread::new(cfg, registry, tools, None);

    let mut handle = thread.submit(make_sub("hi")).await;
    // Give the loop a moment to start streaming, then cancel.
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.cancel();

    let mut saw_aborted = false;
    let mut saw_turn_complete = false;
    while let Some(ev) = handle.next().await {
        match ev.msg {
            EventMsg::TurnAborted(_) => saw_aborted = true,
            EventMsg::TurnComplete(_) => saw_turn_complete = true,
            _ => {}
        }
    }
    assert!(saw_aborted, "expected TurnAborted after cancel");
    assert!(!saw_turn_complete, "cancel should not produce TurnComplete");
}
