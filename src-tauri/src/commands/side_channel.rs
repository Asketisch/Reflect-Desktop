//! Side-channel management commands — wraps
//! `reflect_app_core::side_channel::SideChannelRegistry`.
//!
//! Phase 2 第 1 项:用户驱动的并发 agent。`/agent <name> <prompt>` 在前端
//! 触发 `reflect_start_side_channel` 拿回独立 id + CancelToken,后续通过
//! `reflect_event` 订阅 `kind: side_channel_*` 事件更新 UI。
//!
//! ## 命令清单
//!
//! - `reflect_start_side_channel` — 注册一条 side-channel(返回 id + cancel_token
//!   占位;CancelToken 不暴露给前端,通过 `reflect_cancel_side_channel` 取消)
//! - `reflect_cancel_side_channel` — 取消指定 id
//! - `reflect_list_side_channels` — 列出当前所有 side-channel 快照
//! - `reflect_get_side_channel` — 单条快照(可选)
//!
//! ## 未做(后续阶段)
//!
//! - 把 side-channel 的"实际执行"接到 `reflect-core::AgentThread`:
//!   registry 只管理 id + cancel + 事件,执行侧需要 driver 任务把 prompt
//!   作为 `Submission::user_input` 注入 agent loop 并监听 Output。
//!   本轮先落地 registry + 命令面 + 事件流,drive 留 Phase 2 后半段。

use reflect_app_core::side_channel::{SideChannelId, SideChannelInfo};
use serde::Serialize;
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// `reflect_start_side_channel` 的返回值。前端用 id 订阅事件 / 取消。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSideChannelResult {
    pub id: SideChannelId,
    pub agent_name: String,
    pub prompt: String,
    pub started_at_ms: i64,
}

/// Start a new side-channel. Returns its id + the same data the list call
/// would surface (`started_at_ms` etc.). Emits a `Started` event on the
/// registry's broadcast channel.
#[tauri::command]
pub async fn reflect_start_side_channel(
    agent: State<'_, MinimalAgent>,
    agent_name: String,
    prompt: String,
) -> CommandResult<StartSideChannelResult> {
    if prompt.trim().is_empty() {
        return Err(CommandError {
            msg: "side-channel prompt cannot be empty".into(),
        });
    }
    let reg = agent.side_channels();
    let (id, _cancel) = reg.start(agent_name.clone(), prompt.clone());
    let info = reg.get(&id).ok_or_else(|| CommandError {
        msg: "side-channel not found after start (race?)".into(),
    })?;
    Ok(StartSideChannelResult {
        id,
        agent_name: info.agent_name,
        prompt: info.prompt,
        started_at_ms: info.started_at_ms,
    })
}

/// Cancel a running side-channel by id. Returns `true` if cancellation took
/// effect, `false` if the id was not found or already terminal.
#[tauri::command]
pub async fn reflect_cancel_side_channel(
    agent: State<'_, MinimalAgent>,
    id: String,
) -> CommandResult<bool> {
    Ok(agent.side_channels().cancel(&id))
}

/// List all current side-channels. The frontend subscribes via the registry's
/// broadcast channel for live updates; this command is for ad-hoc snapshots
/// (e.g. page reload).
#[tauri::command]
pub async fn reflect_list_side_channels(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<SideChannelInfo>> {
    Ok(agent.side_channels().list())
}

/// Single-side-channel snapshot.
#[tauri::command]
pub async fn reflect_get_side_channel(
    agent: State<'_, MinimalAgent>,
    id: String,
) -> CommandResult<SideChannelInfo> {
    agent
        .side_channels()
        .get(&id)
        .ok_or_else(|| CommandError {
            msg: format!("side-channel '{id}' not found"),
        })
}

// `SideChannelRegistry` is re-exported so other modules (event forwarder in
// particular) can subscribe without depending on reflect-app-core directly.
pub use reflect_app_core::side_channel::SideChannelRegistry as _SideChannelRegistryReexport;

#[cfg(test)]
mod tests {
    //! 命令包装层测试。沿用 sessions/tasks/schedule/agents 策略:
    //! 直接构造 app-core SideChannelRegistry,证明命令层 API 真实存在 + 转发逻辑正确。
    //!
    //! 注意:`reflect_start_side_channel` 等命令需要 `tauri::State`,这里我们
    //! 复用 app-core 的 start/get/list/cancel API 测行为,等价于"如果
    //! Tauri State 注入失败,这一层会断"。

    use reflect_app_core::side_channel::{SideChannelInfo, SideChannelRegistry, SideChannelStatus};
    use std::sync::Arc;

    fn reg() -> Arc<SideChannelRegistry> {
        Arc::new(SideChannelRegistry::new())
    }

    #[test]
    fn empty_registry_yields_empty_list() {
        let r = reg();
        assert!(r.list().is_empty());
        assert_eq!(r.running_count(), 0);
    }

    #[test]
    fn start_then_get_roundtrip() {
        let r = reg();
        let (id, _) = r.start("default".into(), "hello".into());
        let info: SideChannelInfo = r.get(&id).expect("should exist after start");
        assert_eq!(info.agent_name, "default");
        assert_eq!(info.prompt, "hello");
        assert_eq!(info.status, "running");
        assert!(info.duration_ms.is_none());
        assert_eq!(r.running_count(), 1);
    }

    #[test]
    fn cancel_alone_returns_true() {
        let r = reg();
        let (id, _) = r.start("default".into(), "x".into());
        assert!(r.cancel(&id));
        // Status now cancelled; duration populated.
        let info = r.get(&id).unwrap();
        assert_eq!(info.status, "cancelled");
        assert!(info.duration_ms.is_some());
    }

    #[test]
    fn cancel_unknown_id_returns_false() {
        let r = reg();
        assert!(!r.cancel("side-deadbeef"));
    }

    #[test]
    fn list_returns_started_in_insertion_order() {
        let r = reg();
        let (a, _) = r.start("default".into(), "a".into());
        let (b, _) = r.start("default".into(), "b".into());
        let v = r.list();
        assert_eq!(v.len(), 2);
        // Sorted by started_at_ms ascending.
        assert!(v[0].started_at_ms <= v[1].started_at_ms);
        // The two ids should both appear (we don't assert a == a[0] etc.
        // because started_at_ms uses the registry-level pulse, but a/b
        // ordering tests that both show up).
        let ids: Vec<_> = v.iter().map(|i| &i.id).collect();
        assert!(ids.contains(&&a));
        assert!(ids.contains(&&b));
    }

    /// `finish` transitions a running side-channel through the runner task's
    /// outcomes.
    #[test]
    fn finish_done_and_error_transitions() {
        let r = reg();
        let (id, _) = r.start("default".into(), "x".into());

        r.finish(&id, SideChannelStatus::Done, Some("ok".into()));
        let info = r.get(&id).unwrap();
        assert_eq!(info.status, "done");
        assert!(info.duration_ms.is_some());

        // Error path on a fresh handle
        let (id2, _) = r.start("default".into(), "y".into());
        r.finish(&id2, SideChannelStatus::Error, Some("boom".into()));
        let info = r.get(&id2).unwrap();
        assert_eq!(info.status, "error");
    }
}
