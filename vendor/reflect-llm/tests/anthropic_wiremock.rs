//! Wiremock integration tests for the Anthropic provider.

use futures::StreamExt;
use reflect_llm::providers::{AnthropicClient, AnthropicConfig};
use reflect_llm::{
    ChatEvent, ChatMessage, ChatRequest, ContentBlock, LlmError, ModelClient, SystemBlocks,
    UserContent,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn basic_request() -> ChatRequest {
    ChatRequest {
        model: "claude-3-5-sonnet-latest".into(),
        messages: vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("hi")],
        })],
        tools: vec![],
        system: SystemBlocks::default(),
        temperature: None,
        max_tokens: Some(256),
        top_p: None,
        thinking: None,
        cache_control: vec![],
        metadata: Default::default(),
        stop: vec![],
    }
}

fn make_sse(events: &[&str]) -> String {
    let mut out = String::new();
    for ev in events {
        out.push_str(ev);
        out.push_str("\n\n");
    }
    out
}

#[tokio::test]
async fn anthropic_happy_path_parses_sse() {
    let server = MockServer::start().await;
    let sse = make_sse(&[
        "event: message_start\ndata: {\"message\":{\"id\":\"msg_1\",\"model\":\"claude-3-5-sonnet-latest\",\"usage\":{\"input_tokens\":5,\"output_tokens\":1,\"cache_read_input_tokens\":2}}}",
        "event: content_block_start\ndata: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}",
        "event: content_block_delta\ndata: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}",
        "event: content_block_stop\ndata: {\"index\":0}",
        "event: message_delta\ndata: {\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}",
        "event: message_stop\ndata: {}",
    ]);
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "test-key".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client
        .stream(basic_request(), CancellationToken::new())
        .await
        .unwrap();
    let mut events = Vec::new();
    while let Some(e) = stream.next().await {
        events.push(e.unwrap());
    }
    assert!(matches!(&events[0], ChatEvent::MessageStart { id, .. } if id == "msg_1"));
    // Expect: MessageStart, (text_block_start → no event), ContentDelta("Hello"),
    // (content_block_stop → no event), Usage, MessageStop
    let saw_delta = events
        .iter()
        .any(|e| matches!(e, ChatEvent::ContentDelta(s) if s == "Hello"));
    assert!(saw_delta, "expected ContentDelta(Hello) in {events:?}");
    let saw_usage = events.iter().any(|e| {
        matches!(e, ChatEvent::Usage {
            input_tokens,
            output_tokens,
            cached_tokens,
            cache_write_tokens,
        } if *input_tokens == 5 && *output_tokens == 2 && *cached_tokens == 2 && *cache_write_tokens == 0)
    });
    assert!(saw_usage, "expected Usage(5,2,2,0) in {events:?}");
    assert!(matches!(events.last(), Some(ChatEvent::MessageStop)));
}

#[tokio::test]
async fn anthropic_401_maps_to_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let err = match client
        .stream(basic_request(), CancellationToken::new())
        .await
    {
        Ok(_) => panic!("expected error, got Ok"),
        Err(e) => e,
    };
    assert!(matches!(err, LlmError::Auth));
}

#[tokio::test]
async fn anthropic_529_maps_to_overloaded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(529).set_body_string("overloaded"))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let err = match client
        .stream(basic_request(), CancellationToken::new())
        .await
    {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(matches!(err, LlmError::Overloaded { .. }));
}

#[tokio::test]
async fn anthropic_request_body_has_cache_control() {
    use reflect_llm::request::{SystemBlock, SystemBlocks};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {}\n\n"),
        )
        .mount(&server)
        .await;

    let mut req = basic_request();
    req.system = SystemBlocks(vec![SystemBlock {
        text: "be helpful".into(),
        cache_control: None,
        ephemeral: false,
    }]);

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client.stream(req, CancellationToken::new()).await.unwrap();
    // Drain the stream to ensure the request was sent.
    while stream.next().await.is_some() {}
    let received = server.received_requests().await.unwrap();
    assert!(!received.is_empty());
    let body = String::from_utf8_lossy(&received[0].body);
    assert!(
        body.contains("\"cache_control\""),
        "body missing cache_control: {body}"
    );
    assert!(
        body.contains("\"ttl\":\"5m\""),
        "body missing ttl:5m: {body}"
    );
    let auth = received[0]
        .headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(auth, "k");
}

/// v1.x:系统块带 `CacheTtl::OneHour` 时,出站 body 的 `cache_control.ttl`
/// 应为 `"1h"`(对齐 Anthropic 长生命周期 system prompt 缓存)。这条 wire
/// 映射由 `cache_control_json` 实现;上层 `inject_cache_control` 默认 1h
/// 已在 reflect-prompt 单测覆盖。
#[tokio::test]
async fn anthropic_system_block_ttl_1h_on_wire() {
    use reflect_llm::request::{CacheControl, CacheControlKind, CacheTtl, SystemBlock};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {}\n\n"),
        )
        .mount(&server)
        .await;

    let mut req = basic_request();
    req.system = SystemBlocks(vec![SystemBlock {
        text: "be helpful".into(),
        cache_control: Some(CacheControl {
            kind: CacheControlKind::Ephemeral,
            ttl: Some(CacheTtl::OneHour),
        }),
        ephemeral: false,
    }]);

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client.stream(req, CancellationToken::new()).await.unwrap();
    while stream.next().await.is_some() {}
    let received = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&received[0].body);
    assert!(
        body.contains("\"ttl\":\"1h\""),
        "system 块 ttl 应为 1h;got: {body}"
    );
}

#[tokio::test]
async fn anthropic_parses_cache_creation_input_tokens() {
    // M8: capture `cache_creation_input_tokens` from the message_start event
    // and surface it as `Usage.cache_write_tokens`.
    let server = MockServer::start().await;
    let sse = make_sse(&[
        "event: message_start\ndata: {\"message\":{\"id\":\"msg_2\",\"model\":\"claude-3-5-sonnet-latest\",\"usage\":{\"input_tokens\":12,\"output_tokens\":1,\"cache_read_input_tokens\":3,\"cache_creation_input_tokens\":7}}}",
        "event: content_block_start\ndata: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}",
        "event: content_block_delta\ndata: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}",
        "event: content_block_stop\ndata: {\"index\":0}",
        "event: message_delta\ndata: {\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}",
        "event: message_stop\ndata: {}",
    ]);
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client
        .stream(basic_request(), CancellationToken::new())
        .await
        .unwrap();
    let mut events = Vec::new();
    while let Some(e) = stream.next().await {
        events.push(e.unwrap());
    }
    let saw_usage = events.iter().any(|e| {
        matches!(e, ChatEvent::Usage {
            input_tokens,
            output_tokens,
            cached_tokens,
            cache_write_tokens,
        } if *input_tokens == 12
            && *output_tokens == 2
            && *cached_tokens == 3
            && *cache_write_tokens == 7)
    });
    assert!(
        saw_usage,
        "expected Usage(12,2,3,7) — cache_creation_input_tokens should surface as cache_write_tokens; got {events:?}"
    );
}

#[tokio::test]
async fn anthropic_request_attaches_cache_control_to_last_tools() {
    // M8: when `req.metadata["cache_break_tool"] = "true"`, the last N tool
    // defs in the outgoing body should each carry `cache_control: ephemeral`.
    use reflect_llm::request::{SystemBlock, SystemBlocks, ToolSpec};
    use serde_json::json;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {}\n\n"),
        )
        .mount(&server)
        .await;

    let mut req = basic_request();
    req.system = SystemBlocks(vec![SystemBlock {
        text: "be helpful".into(),
        cache_control: None,
        ephemeral: false,
    }]);
    // 5 tools — the last 3 should get cache_control attached.
    for (name, _) in [
        ("read", 0),
        ("write", 1),
        ("edit", 2),
        ("grep", 3),
        ("glob", 4),
    ] {
        req.tools.push(ToolSpec::Function {
            name: name.into(),
            description: "".into(),
            parameters: json!({"type": "object"}),
        });
    }
    req.metadata
        .insert("cache_break_tool".to_string(), "true".to_string());

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client.stream(req, CancellationToken::new()).await.unwrap();
    while stream.next().await.is_some() {}
    let received = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&received[0].body);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let tools = v["tools"].as_array().expect("tools[] present");
    assert_eq!(tools.len(), 5);
    // First 2 tools: no cache_control.
    for (i, t) in tools.iter().enumerate().take(2) {
        assert!(
            t.get("cache_control").is_none(),
            "tool[{i}] should NOT have cache_control, got {t}"
        );
    }
    // Last 3 tools: cache_control.ephemeral.
    for (i, t) in tools.iter().enumerate().skip(2) {
        let cc = t.get("cache_control").expect("cache_control present");
        assert_eq!(cc["type"], "ephemeral", "tool[{i}] cache_control.type");
        assert_eq!(cc["ttl"], "5m", "tool[{i}] cache_control.ttl");
    }
}

#[tokio::test]
async fn anthropic_request_attaches_cache_control_to_prefix_anchor_message() {
    // M8: when `req.cache_control` carries a `CacheBreak` entry pointing at
    // message index N, the last content block of that message gets
    // `cache_control: ephemeral`.
    use reflect_llm::CacheTtl;
    use reflect_llm::request::CacheBreak;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_stop\ndata: {}\n\n"),
        )
        .mount(&server)
        .await;

    let mut req = basic_request();
    req.messages = vec![
        ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("first user turn")],
        }),
        ChatMessage::Assistant(reflect_llm::AssistantContent {
            text: Some("first assistant".into()),
            tool_calls: vec![],
            thinking: None,
        }),
        ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("second user turn — anchor")],
        }),
    ];
    // Anchor the cache_control breakpoint on the last user message (idx 2).
    req.cache_control.push(CacheBreak {
        after_message_index: 2,
        ttl: CacheTtl::FiveMinutes,
    });

    let client = AnthropicClient::new(AnthropicConfig {
        api_key: "k".into(),
        base_url: Some(server.uri()),
        timeout: Duration::from_secs(5),
    })
    .unwrap();
    let mut stream = client.stream(req, CancellationToken::new()).await.unwrap();
    while stream.next().await.is_some() {}
    let received = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&received[0].body);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let messages = v["messages"].as_array().expect("messages[] present");
    assert_eq!(messages.len(), 3);
    // First two messages: no cache_control on any block.
    for (i, m) in messages.iter().enumerate().take(2) {
        let blocks = m["content"].as_array().expect("content[] present");
        for (j, b) in blocks.iter().enumerate() {
            assert!(
                b.get("cache_control").is_none(),
                "messages[{i}].content[{j}] should NOT have cache_control, got {b}"
            );
        }
    }
    // Third message: last content block has cache_control.
    let anchor_blocks = messages[2]["content"].as_array().unwrap();
    let last = anchor_blocks.last().unwrap();
    let cc = last
        .get("cache_control")
        .expect("anchor block has cache_control");
    assert_eq!(cc["type"], "ephemeral");
    assert_eq!(cc["ttl"], "5m");
}
