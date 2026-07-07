//! Event 转发 —— 把 `MinimalAgent::subscribe_session()` 收的 Event 通过
//! `app.emit("reflect_event", &event)` 推到前端。
//!
//! M1.2 minimal 版本: 单一 Tauri event name `reflect_event`,
//! 完整 Event (含 id + EventMsg) 作为 payload。前端 `services/tauri.ts`
//! 用 `msg.type` (serde discriminator `type`) 分派到对应 reducer。

use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use reflect_protocol::Event;

/// Forward task —— 在 Tauri setup 阶段 spawn 一次,持续到 AppHandle drop。
///
/// M1.2 minimal: 单一"reflect_event"频道。所有客户端侧订阅都通过这一名
/// 收 payload,然后按 EventMsg.type 派发。M2.x 不再变更。
pub async fn forward_agent_events(app: AppHandle, mut rx: mpsc::Receiver<Event>) {
    tracing::info!("[reflect-gui] forward_agent_events started");
    while let Some(event) = rx.recv().await {
        if let Err(e) = app.emit("reflect_event", &event) {
            tracing::error!("[reflect-gui] emit reflect_event failed: {e}");
        }
    }
    tracing::warn!("[reflect-gui] forward_agent_events exited");
}
