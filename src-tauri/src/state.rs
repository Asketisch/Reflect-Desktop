//! `AppState` —— M1.2 协议桥的最小后端状态。
//!
//! 设计目标：证明 `Submission`→`Event` IPC 通路接通,而非完整复刻 TUI 后端。
//!
//! ## 进度
//!
//! - [x] **M1.2 协议桥**:
//!     - MinimalAgent 收到 Submission 后 emit 一段模拟流("Got it. (stub) You said: ...").
//!     - 实际接入真实 `reflect_core::AgentThread` 留待 M2.x(需要 Model 路由 + 22 builtin
//!       tools + secrets 注入 + storage adapter,1-2 天工作量),可由 docs/gui/03-architecture.md
//!       §5 设计的 AppState 直接替换。

use std::sync::Arc;

use parking_lot::Mutex as ParkingMutex;
use reflect_protocol::{
    AgentMessage, ApprovalPolicy, Event, EventMsg, SandboxPolicy, SessionConfiguredEvent,
    Submission, ThreadId, TokenUsage, TurnCompleteEvent, TurnStartedEvent, TurnStatus,
};
use std::sync::Mutex as StdMutex;
use tokio::sync::mpsc;

/// 主结构：MinimalAgent 拥有一个 submission 通道与若干 session 级订阅者。
///
/// 接口形状模仿 `reflect_core::AgentThread`
/// (`crates/reflect-core/src/agent_thread.rs:25-43`),便于 M2.x 直接替换为真实 AgentThread。
#[derive(Clone)]
pub struct MinimalAgent {
    inner: Arc<MinimalAgentInner>,
}

struct MinimalAgentInner {
    sub_tx: mpsc::Sender<Submission>,
    /// 实际 consuming submission 的 receiver —— 由 `start()` 在 Tauri runtime 内
    /// `tokio::spawn` 一次性拿走,避免 `manage()` 同步上下文中 panic。
    sub_rx: StdMutex<Option<mpsc::Receiver<Submission>>>,
    /// session-level subscribers —— `SessionConfigured` / `ShutdownComplete` 等
    /// 生命周期事件 fan-out 到这里。M1.2 简化为所有 event 都通过 session_subs 派发,
    /// 真实 AgentThread 在 src-tauri::state 替换时再区分 per-turn / session sub。
    session_subs: ParkingMutex<Vec<mpsc::Sender<Event>>>,
}

impl MinimalAgent {
    /// 构造最小 Agent —— *只* 创建 channel,**延迟** submission loop 到 `start()`。
    pub fn spawn() -> Self {
        let (sub_tx, sub_rx) = mpsc::channel::<Submission>(64);
        let inner = Arc::new(MinimalAgentInner {
            sub_tx,
            sub_rx: StdMutex::new(Some(sub_rx)),
            session_subs: ParkingMutex::new(Vec::new()),
        });
        Self { inner }
    }

    /// 启动 submission loop —— 必须在 Tauri `setup` 闭包内调用(那时候 Tauri 已经
    /// 初始化了它的 `tauri::async_runtime`,我们 `Spawn` 用的是 `tauri::async_runtime::spawn`,
    /// 这样无论 thread 上下文如何都能进到 Tauri 自己的 tokio runtime)。
    pub fn start(&self) {
        let rx = self.inner.sub_rx.lock().expect("sub_rx poisoned").take();
        if let Some(rx) = rx {
            tauri::async_runtime::spawn(minimal_submission_loop(self.inner.clone(), rx));
        }
    }

    /// 发送 Submission 到后端。
    pub async fn submit(&self, submission: Submission) -> anyhow::Result<()> {
        self.inner.sub_tx.send(submission).await?;
        Ok(())
    }

    /// 订阅 session 级事件。
    pub fn subscribe_session(&self) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel::<Event>(16);
        let mut subs = self.inner.session_subs.lock();
        subs.push(tx);
        rx
    }

    /// 中断当前 turn —— MinimalAgent 无 turn 状态,空操作。
    pub fn interrupt(&self) {
        // stub
    }
}

/// 模拟 submission loop：收到 Submission 后 emit 一组 lifecycle + 流式 AgentMessageDelta。
async fn minimal_submission_loop(
    inner: Arc<MinimalAgentInner>,
    mut sub_rx: mpsc::Receiver<Submission>,
) {
    while let Some(submission) = sub_rx.recv().await {
        handle_submission(&inner, submission).await;
    }
}

async fn handle_submission(inner: &MinimalAgentInner, submission: Submission) {
    // 1. SessionConfigured (lifecycle event, 仅第一次发送)
    static FIRST_SUB: std::sync::OnceLock<ParkingMutex<bool>> = std::sync::OnceLock::new();
    let first_lock = FIRST_SUB.get_or_init(|| ParkingMutex::new(false));
    {
        let mut sent = first_lock.lock();
        if !*sent {
            *sent = true;
            let event = Event::new(
                "",
                EventMsg::SessionConfigured(SessionConfiguredEvent {
                    session_id: ThreadId(uuid::Uuid::new_v4()),
                    model: "stub-model".into(),
                    provider: "stub-provider".into(),
                    approval_policy: ApprovalPolicy::default(),
                    sandbox_policy: SandboxPolicy::default(),
                    context_window_size: None,
                }),
            );
            broadcast(inner, event);
        }
    }

    // 2. TurnStarted
    let turn_started = Event::new(
        submission.id.clone(),
        EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: reflect_protocol::item::TurnId(uuid::Uuid::new_v4()),
            user_message_id: None,
        }),
    );
    broadcast(inner, turn_started);

    // 3. 流式 AgentMessageDelta —— 拼出 "Got it. (stub) You said: ..."
    let text = match &submission.op {
        reflect_protocol::Op::UserInput { items, .. } => items
            .iter()
            .filter_map(|i| match i {
                reflect_protocol::UserInputItem::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    };

    let reply = format!("Got it. (stub) You said: {text}");
    for chunk in split_streams(&reply) {
        let ev = Event::new(
            submission.id.clone(),
            EventMsg::AgentMessageDelta(reflect_protocol::AgentMessageDelta { delta: chunk }),
        );
        broadcast(inner, ev);
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }

    // 4. AgentMessage (final assembled)
    let final_msg = Event::new(
        submission.id.clone(),
        EventMsg::AgentMessage(AgentMessage { text: reply.clone() }),
    );
    broadcast(inner, final_msg);

    // 5. TurnComplete
    let complete = Event::new(
        submission.id.clone(),
        EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: reflect_protocol::item::TurnId(uuid::Uuid::new_v4()),
            usage: TokenUsage::default(),
            status: TurnStatus::Success,
        }),
    );
    broadcast(inner, complete);
}

/// 把 Event 派发给所有 session subscribers(M1.2 简化:`try_send` 同步决策)。
fn broadcast(inner: &MinimalAgentInner, event: Event) {
    let mut subs = inner.session_subs.lock();
    let mut alive = Vec::with_capacity(subs.len());
    for tx in subs.drain(..) {
        let sender: mpsc::Sender<Event> = tx;
        match sender.try_send(event.clone()) {
            Ok(()) => alive.push(sender),
            Err(_) => {
                // 订阅者通道已满或断开 —— 剔除
            }
        }
    }
    *subs = alive;
}

/// 把字符串切成 ~4 字符小段,模拟流式 chunk。
fn split_streams(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    for chunk in chars.chunks(4) {
        out.push(chunk.iter().collect());
    }
    out
}
