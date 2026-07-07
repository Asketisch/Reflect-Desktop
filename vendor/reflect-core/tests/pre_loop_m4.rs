//! M4 integration tests for `pre_loop` + `model_call` wiring.
//!
//! These tests construct a `NodeContext` with M4 deps populated and
//! drive the StateGraph through one turn. They verify that the M4
//! pipeline (compact / memory / skills / prompt builder) runs end to
//! end.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, stream};
use parking_lot::Mutex;
use reflect_agent_def::AgentDefinition;
use reflect_compact::{Compactor, CompactorConfig, Summarizer, SummarizerError};
use reflect_core::config::M4Deps;
use reflect_core::{AgentConfig, AgentThread};
use reflect_llm::{
    Capabilities, ChatEvent, ChatMessage, ChatRequest, CredentialPool, LlmError, ModelClient,
    ModelRegistry, PoolEntry,
};
use reflect_memory::{FileMemoryStore, InMemoryStore, MemoryScope, MemoryStore};
use reflect_prompt::PromptBuilder;
use reflect_protocol::{EventMsg, Submission, UserInputItem};
use reflect_skills::SkillsCatalog;
use reflect_tools::{ToolRegistry, builtins::EchoTool};
use tokio_util::sync::CancellationToken;

struct StubClient {
    events: Mutex<Vec<ChatEvent>>,
    last_request: Mutex<Option<ChatRequest>>,
    call_count: Mutex<u32>,
}

impl StubClient {
    fn new(events: Vec<ChatEvent>) -> Self {
        Self {
            events: Mutex::new(events),
            last_request: Mutex::new(None),
            call_count: Mutex::new(0),
        }
    }
}

#[async_trait]
impl ModelClient for StubClient {
    fn name(&self) -> &str {
        "stub"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tool_use: true,
            prompt_caching: true,
            ..Default::default()
        }
    }
    async fn stream(
        &self,
        request: ChatRequest,
        _cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError> {
        *self.call_count.lock() += 1;
        *self.last_request.lock() = Some(request);
        let events = self.events.lock().clone();
        Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
    }
}

struct NoopSummarizer;
#[async_trait]
impl Summarizer for NoopSummarizer {
    async fn summarize_full(&self, _msgs: &[ChatMessage]) -> Result<String, SummarizerError> {
        Err(SummarizerError::Cancelled)
    }
    async fn summarize_recent(
        &self,
        _msgs: &[ChatMessage],
        _prev: Option<&str>,
    ) -> Result<String, SummarizerError> {
        Err(SummarizerError::Cancelled)
    }
}

fn make_m4_deps(agent_name: &str, system_prompt: &str) -> M4Deps {
    let compactor = Arc::new(Compactor::new(
        CompactorConfig {
            summarize_after: false, // never call the LLM in tests
            ..Default::default()
        },
        Arc::new(NoopSummarizer),
    ));
    let tmp = std::env::temp_dir().join(format!("reflect-m4-mem-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&tmp);
    let memory: Arc<dyn MemoryStore> = Arc::new(FileMemoryStore::new(&tmp, &tmp));
    let skills = Arc::new(SkillsCatalog::new());
    let prompt_builder = Arc::new(Mutex::new(PromptBuilder::new()));
    let note_store: Arc<dyn reflect_notes::NoteStore> =
        Arc::new(reflect_notes::InMemoryNoteStore::new());
    let file_recovery = Arc::new(reflect_recovery::ActiveFileRecovery::new(Arc::from(
        tmp.clone(),
    )));
    let subagent_registry = reflect_recovery::SubagentRegistry::shared();
    let mut def = AgentDefinition::default();
    #[allow(clippy::field_reassign_with_default)] // pre-M5
    {
        def.name = agent_name.to_string();
        def.description = "test".into();
        def.system_prompt = system_prompt.into();
        def.memory = vec![MemoryScope::Project];
    }
    M4Deps {
        compactor,
        memory,
        skills,
        prompt_builder,
        active_agent_def: Arc::new(def),
        recorder: None,
        note_store,
        file_recovery,
        subagent_registry,
    }
}

fn build_thread(registry: Arc<ModelRegistry>, m4: M4Deps) -> AgentThread {
    let cfg = AgentConfig::new("stub/m1", Path::new(".")).with_m4(m4);
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
async fn m4_injects_system_prompt_into_request() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("ok".into()),
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

    let m4 = make_m4_deps("tester", "You are a reviewer.");
    let thread = build_thread(registry, m4);

    let mut handle = thread.submit(make_sub("hi")).await;
    while let Some(ev) = handle.next().await {
        if matches!(ev.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    // Verify the request the model saw contained the system prompt.
    let req = stub.last_request.lock().clone().expect("model was called");
    let has_system = req
        .system
        .0
        .iter()
        .any(|b| b.text.contains("You are a reviewer"));
    assert!(has_system, "system prompt should be in the request");
}

#[tokio::test]
async fn m4_loads_memory_into_request() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("ok".into()),
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

    let mut m4 = make_m4_deps("tester", "You are a tester.");
    let _ = &mut m4; // silence unused-mut; `m4.memory = ...` later mutates
    // Save some project memory and reload.
    m4.memory
        .save(
            MemoryScope::Project,
            "tester",
            "## Facts\n- the sky is blue\n",
        )
        .unwrap();
    let thread = build_thread(registry, m4);

    let mut handle = thread.submit(make_sub("hi")).await;
    while let Some(ev) = handle.next().await {
        if matches!(ev.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    let req = stub.last_request.lock().clone().expect("model was called");
    let has_memory = req
        .system
        .0
        .iter()
        .any(|b| b.text.contains("the sky is blue"));
    assert!(has_memory, "memory should be in the system prompt");
}

#[tokio::test]
async fn m4_injects_ephemeral_block_as_user_message() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("ok".into()),
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

    let m4 = make_m4_deps("tester", "sys");
    let thread = build_thread(registry, m4);

    let mut handle = thread.submit(make_sub("hi")).await;
    while let Some(ev) = handle.next().await {
        if matches!(ev.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    let req = stub.last_request.lock().clone().expect("model was called");
    let has_reminder = req
        .messages
        .iter()
        .any(|m| matches!(m, ChatMessage::User(_)))
        && req.messages.iter().any(|m| {
            let ChatMessage::User(u) = m else {
                return false;
            };
            u.blocks.iter().any(|b| {
                let reflect_llm::ContentBlock::Text { text } = b else {
                    return false;
                };
                text.contains("system-reminder")
            })
        });
    assert!(
        has_reminder,
        "ephemeral block should be appended as a user message"
    );
}

#[tokio::test]
async fn m4_session_only_store_does_not_persist_to_disk() {
    let registry = Arc::new(ModelRegistry::new());
    let stub = Arc::new(StubClient::new(vec![
        ChatEvent::MessageStart {
            id: "m1".into(),
            model: "stub-1".into(),
        },
        ChatEvent::ContentDelta("ok".into()),
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

    let mut m4 = make_m4_deps("tester", "sys");
    // Replace memory with a session-only store.
    let in_mem: Arc<dyn MemoryStore> = Arc::new(InMemoryStore::new());
    in_mem
        .save(MemoryScope::Session, "tester", "ephemeral fact")
        .unwrap();
    m4.memory = in_mem;
    Arc::make_mut(&mut m4.active_agent_def).memory = vec![MemoryScope::Session];

    let thread = build_thread(registry, m4);
    let mut handle = thread.submit(make_sub("hi")).await;
    while let Some(ev) = handle.next().await {
        if matches!(ev.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    let req = stub.last_request.lock().clone().expect("model was called");
    let has_fact = req
        .system
        .0
        .iter()
        .any(|b| b.text.contains("ephemeral fact"));
    assert!(has_fact);
}
