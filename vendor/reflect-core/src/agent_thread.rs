//! `AgentThread` — owns the submission channel and per-turn / per-session
//! event fan-out.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use reflect_llm::SharedModelRegistry;
use reflect_protocol::{Event, Submission};
use reflect_tools::{Sanitizer, ToolExecutionQueue, ToolRegistry};

use crate::config::AgentConfig;
use crate::submission_loop::submission_loop;
use crate::turn::TurnHandle;

/// Capacity for a single session-event subscriber. Lifecycle events
/// (`SessionConfigured`, `ShutdownComplete`) are rare so a small buffer
/// suffices; a slow subscriber drops the oldest pending event.
const SESSION_SUB_CAPACITY: usize = 16;

/// M1 thread — single submission at a time. M2 supports concurrent turns.
pub struct AgentThread {
    cfg: AgentConfig,
    registry: SharedModelRegistry,
    tools: Arc<ToolRegistry>,
    /// M6: owned here so `register_hook` can mutate the queue's
    /// `HookEngine`. Also passed (as `Arc` clone) into the submission
    /// loop task.
    tools_queue: Arc<ToolExecutionQueue>,
    sub_tx: mpsc::Sender<Submission>,
    cancel: CancellationToken,
    /// `Submission.id` → event channel for that submission. The submission
    /// loop removes the entry when the turn completes, which drops the
    /// `Sender` and closes the `Receiver` in the caller's `TurnHandle`.
    turn_subs: Arc<Mutex<HashMap<String, mpsc::Sender<Event>>>>,
    /// M6: session-level subscribers. `SessionConfigured` and
    /// `ShutdownComplete` are fanned out here in addition to the per-turn
    /// channel so a TUI can pick them up without tying them to a submission.
    session_subs: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
}

impl AgentThread {
    /// 构造 `AgentThread`。
    ///
    /// `sanitizer` 控制工具输出密钥脱敏:
    /// - `Some(arc)` — 使用调用方注入的 sanitizer(典型来源:
    ///   `Sanitizer::from_config(&reflect_config::SanitizeSection)`)。
    /// - `None` — fallback 到 `Sanitizer::with_defaults()`(10 个默认
    ///   pattern + `[REDACTED]`),与历史行为一致。
    ///
    /// 加这个参数是为了把 `~/.reflect/config.toml [sanitize]` 段真正
    /// 接到 queue 内部 `Ok(Ok(_))` 分支的脱敏 pass 上 —— review 2026-06-30
    /// 之前的版本硬编码 `with_defaults`,用户的 `enabled = false` /
    /// `marker = "..."` / `extra_patterns = [...]` 全部死信。
    pub fn new(
        cfg: AgentConfig,
        registry: SharedModelRegistry,
        tools: Arc<ToolRegistry>,
        sanitizer: Option<Arc<Sanitizer>>,
    ) -> Self {
        let cancel = cfg.cancel.clone();
        let (sub_tx, sub_rx) = mpsc::channel::<Submission>(64);
        let turn_subs: Arc<Mutex<HashMap<String, mpsc::Sender<Event>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let session_subs: Arc<Mutex<Vec<mpsc::Sender<Event>>>> = Arc::new(Mutex::new(Vec::new()));
        let mut base_ctx = cfg.shared_tool_context();
        base_ctx.cancel = cfg.cancel.clone();
        let tools_queue = Arc::new(match sanitizer {
            Some(arc) => ToolExecutionQueue::with_sanitizer(
                tools.clone(),
                Arc::new(reflect_hooks::HookEngine::new()),
                base_ctx,
                arc,
            ),
            None => ToolExecutionQueue::with_defaults(
                tools.clone(),
                Arc::new(reflect_hooks::HookEngine::new()),
                base_ctx,
            ),
        });

        // Spawn submission_loop (it owns sub_rx + the fan-out handles).
        let turn_subs_c = turn_subs.clone();
        let session_subs_c = session_subs.clone();
        let registry_c = registry.clone();
        let tools_c = tools.clone();
        let cfg_c = cfg.clone();
        let tools_queue_for_loop = tools_queue.clone();
        tokio::spawn(async move {
            submission_loop(
                sub_rx,
                turn_subs_c,
                session_subs_c,
                cfg_c,
                registry_c,
                tools_c,
                tools_queue_for_loop,
            )
            .await;
        });

        Self {
            cfg,
            registry,
            tools,
            tools_queue,
            sub_tx,
            cancel,
            turn_subs,
            session_subs,
        }
    }

    /// Submit a `Submission` and obtain a `TurnHandle` for its events.
    /// The submission's `id` is used as the key for per-turn event routing.
    pub async fn submit(&self, sub: Submission) -> TurnHandle {
        let (tx, rx) = mpsc::channel::<Event>(64);
        self.turn_subs.lock().insert(sub.id.clone(), tx);
        // Forward the submission; if the loop is gone, the channel will close.
        let _ = self.sub_tx.send(sub).await;
        TurnHandle::new(rx)
    }

    /// v1.2 P1-2:clone 一份 submission sender 给外部调度器(cron driver),
    /// 让它能向 agent loop 注入 `Submission::user_input`。sender 是
    /// `mpsc::Sender`(clone 廉价、与 `submit` 共享同一 channel);loop
    /// 关闭后 send 返回 Err,调用方应忽略。
    ///
    /// 典型用法:`reflect-exec` 在 `AgentThread` 构造后取 sender,注入
    /// `CronScheduler`,driver 到期时 `tx.send(Submission::user_input(p))`。
    pub fn submission_sender(&self) -> mpsc::Sender<Submission> {
        self.sub_tx.clone()
    }

    /// Subscribe to thread-scoped lifecycle events (`SessionConfigured`,
    /// `ShutdownComplete`). The returned receiver closes when the thread
    /// shuts down. Multiple subscribers are independent; each receives a copy.
    pub fn subscribe_session(&self) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel::<Event>(SESSION_SUB_CAPACITY);
        self.session_subs.lock().push(tx);
        rx
    }

    pub fn config(&self) -> &AgentConfig {
        &self.cfg
    }

    pub fn registry(&self) -> &SharedModelRegistry {
        &self.registry
    }

    pub fn tools(&self) -> &Arc<ToolRegistry> {
        &self.tools
    }

    /// Register a hook on the thread's shared `HookEngine`. The hook
    /// receives every `PreToolUse` / `PostToolUse` / `PostToolUseFailure`
    /// event the tool queue processes.
    pub fn hook_engine(&self) -> Arc<reflect_hooks::HookEngine> {
        self.tools_queue.hook_engine().clone()
    }

    pub fn register_hook<H: reflect_hooks::Hook + 'static>(&self, hook: H) {
        self.tools_queue.register_hook(hook);
    }

    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }
}
