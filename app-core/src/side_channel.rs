//! Side-channel agent 编排。
//!
//! **Side-channel** 是由用户驱动、独立于
//! 主 agent 轮次生命周期的并发 agent 运行。
//!
//! ## 与 `Task` 工具的区别（模型驱动的子 agent、`reflect-subagent`）
//!
//! | | Side-channel | Subagent (`Task`) |
//! |---|---|---|
//! | 触发方式 | 用户输入 `/agent <name> <prompt>` | 模型调用 `Task` 工具 |
//! | 并发性 | 与主 agent 并发运行 | 阻塞主轮次 |
//! | 主历史 | 不受影响 | 工具结果写入主历史 |
//! | 取消 | 独立的 `CancelToken`——主窗口 `Cmd-C` 不会终止它 | 继承父级取消信号 |
//! | UI | 专用标签页中的 `chat_side_channel_*` event | 单个 `Task` 工具指示器 |
//!
//! ## Registry 结构
//!
//! 本模块提供进程级的 `SideChannelRegistry` 和轻量级的
//! `SideChannelHandle`（每次运行一个）。Registry 将稳定 id
//!（`side-<8-hex>`）映射到 handle。每个 handle 拥有独立的 `CancelToken`，因此
//! 主 agent 的取消 token 不会传递到 side-channel。
//!
//! ## 视图侧发送
//!
//! GUI 层可通过 `subscribe_events()` 订阅——这是一个 `broadcast::Receiver`，
//! 每次状态变化（`Started` / `Output` /
//! `Done` / `Error` / `Cancelled`）都会产生一个 `SideChannelEvent`。当前桌面 Tauri 外壳将该
//! receiver 接入现有的 session-event 转发器，使前端获得
//! 已在使用的同一 `reflect_event` 通道。

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// 稳定的 handle 标识符（`side-<8 hex>`）。
pub type SideChannelId = String;

/// side-channel 的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SideChannelStatus {
    /// side-channel 正在执行。
    Running,
    /// side-channel 自然完成（成功或结果为空）。
    Done,
    /// side-channel 已被用户取消。
    Cancelled,
    /// side-channel 因错误退出。
    Error,
}

impl SideChannelStatus {
    /// `"running"` / `"done"` / `"cancelled"` / `"error"`。
    pub fn as_str(self) -> &'static str {
        match self {
            SideChannelStatus::Running => "running",
            SideChannelStatus::Done => "done",
            SideChannelStatus::Cancelled => "cancelled",
            SideChannelStatus::Error => "error",
        }
    }
}

/// Registry event 流中的一项。
///
/// 状态变化（Started / Done / Cancelled /
/// Error）时发送给所有订阅者。runner task 会在 side-channel
/// 产生内容时发送 `Output` event（目前仅在完成时发送——见 `run_once`
/// 的说明；后续将支持增量输出流）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SideChannelEvent {
    /// side-channel 已启动；包含其 id 和所选 agent 名称。
    Started {
        /// 侧通道 id（`side-<8hex>`）。
        id: SideChannelId,
        /// 用户选择的 agent 定义名称。
        agent_name: String,
        /// 提交给 side-channel 的初始 prompt。
        prompt: String,
        /// side-channel 注册时的 UNIX epoch 毫秒时间戳。
        started_at_ms: i64,
    },
    /// side-channel 已正常完成。可选的简短 `summary` 行用于
    /// chats UI 在频道标题中显示。
    Done {
        /// side-channel id。
        id: SideChannelId,
        /// 运行完成时的 UNIX epoch 毫秒时间戳。
        finished_at_ms: i64,
        /// 可选的 summary 行（最后一条 assistant 消息，或为空）。
        summary: Option<String>,
    },
    /// side-channel 已被取消（`reflect_cancel_side_channel` 或其自身的
    /// token 触发）。
    Cancelled {
        /// side-channel id。
        id: SideChannelId,
        /// 取消生效时的 UNIX epoch 毫秒时间戳。
        finished_at_ms: i64,
    },
    /// side-channel 因错误退出（保留错误消息）。
    Error {
        /// side-channel id。
        id: SideChannelId,
        /// 记录错误时的 UNIX epoch 毫秒时间戳。
        finished_at_ms: i64,
        /// 便于人类阅读的错误消息。
        message: String,
    },
}

/// side-channel 的公开视图快照（发送到前端 / IPC）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideChannelInfo {
    /// 稳定的 id。
    pub id: SideChannelId,
    /// agent 定义名称。
    pub agent_name: String,
    /// 初始 prompt（按快照大小截断，不影响存储）。
    pub prompt: String,
    /// side-channel 注册时的 UNIX epoch 毫秒时间戳。
    pub started_at_ms: i64,
    /// 当前状态（`"running"` / `"done"` / `"cancelled"` / `"error"`）。
    pub status: String,
    /// 持续时间（毫秒）；运行中为 `null`。
    pub duration_ms: Option<i64>,
}

/// Registry 中保存的单个 side-channel 状态。会克隆用于 IPC 快照。
pub(crate) struct SideChannelHandle {
    /// agent 定义名称。
    pub agent_name: String,
    /// 初始 prompt。
    pub prompt: String,
    /// 用于计算持续时间的单调时钟起始时间。
    pub started_at: Instant,
    /// 用于 IPC 序列化的墙上时钟起始时间（UNIX_EPOCH 毫秒）。创建时
    /// 计算一次，确保即使前端在运行结束很久后获取快照，也能得到稳定的 epoch 时间戳。
    pub started_at_epoch_ms: i64,
    /// 当前状态。
    pub status: SideChannelStatus,
    /// 独立的取消 token（不是主 token 的子级）。调用方可触发
    /// `cancel.cancel()`，仅停止此 side-channel。
    pub cancel: CancellationToken,
}

impl SideChannelHandle {
    fn snapshot(&self, id: &str) -> SideChannelInfo {
        let duration_ms = if matches!(self.status, SideChannelStatus::Running) {
            None
        } else {
            Some(self.started_at.elapsed().as_millis() as i64)
        };
        SideChannelInfo {
            id: id.to_string(),
            agent_name: self.agent_name.clone(),
            prompt: self.prompt.clone(),
            started_at_ms: self.started_at_epoch_ms,
            status: self.status.as_str().to_string(),
            duration_ms,
        }
    }
}

/// 所有 side-channel 的进程级 Registry。
#[derive(Clone)]
pub struct SideChannelRegistry {
    inner: Arc<Mutex<HashMap<SideChannelId, SideChannelHandle>>>,
    events: broadcast::Sender<SideChannelEvent>,
    /// 进程级计数器，确保即使两次启动发生在同一墙上时钟毫秒内，id 也唯一。
    /// 它与 id 中的时间戳部分配合，使我们仍可根据 `started_at_ms` 推导顺序。
    next_seq: Arc<AtomicU32>,
}

impl Default for SideChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SideChannelRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SideChannelRegistry")
            .field("count", &self.inner.lock().len())
            .finish_non_exhaustive()
    }
}

impl SideChannelRegistry {
    /// 使用默认 broadcast 容量构造 Registry。
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// 使用指定的 broadcast channel 容量构造 Registry。
    pub fn with_capacity(cap: usize) -> Self {
        let (events, _) = broadcast::channel(cap);
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            events,
            next_seq: Arc::new(AtomicU32::new(1)),
        }
    }

    /// 订阅 event 流。
    pub fn subscribe_events(&self) -> broadcast::Receiver<SideChannelEvent> {
        self.events.subscribe()
    }

    /// 插入新的 side-channel。返回分配的 id 和
    /// 调用方必须持有的取消 handle，并发送 `Started` event。
    pub fn start(
        &self,
        agent_name: String,
        prompt: String,
    ) -> (SideChannelId, CancellationToken) {
        // 计算新的 id；发生冲突时使用进程唯一的 seq 重试。
        let epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let mut id = format!("side-{:08x}", (epoch_ms as u32).wrapping_mul(2654435761));
        let mut handle = SideChannelHandle {
            agent_name: agent_name.clone(),
            prompt: prompt.clone(),
            started_at: Instant::now(),
            started_at_epoch_ms: epoch_ms,
            status: SideChannelStatus::Running,
            cancel: CancellationToken::new(),
        };
        let cancel = handle.cancel.clone();
        let mut g = self.inner.lock();
        if g.contains_key(&id) {
            // 极少见：同一毫秒内的两次启动产生了相同哈希。使用
            // 下一个序列号（进程唯一且单调递增）加盐，从而
            // 无论时序多紧密都不会冲突。
            let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
            id = format!("side-{:08x}", (epoch_ms as u32).wrapping_add(seq));
            handle.started_at_epoch_ms += seq as i64;
        } else {
            // 正常路径也递增 seq，使并发冲突重试时得到
            // 严格递增的盐值。
            self.next_seq.fetch_add(1, Ordering::Relaxed);
        }
        g.insert(id.clone(), handle);
        drop(g);
        let _ = self.events.send(SideChannelEvent::Started {
            id: id.clone(),
            agent_name,
            prompt,
            started_at_ms: epoch_ms,
        });
        (id, cancel)
    }

    /// 按 id 取消 side-channel。触发取消 token 并更新状态。
    /// 找到并取消 side-channel 时返回 `true`。
    pub fn cancel(&self, id: &str) -> bool {
        let (started_at_epoch_ms,) = {
            let mut g = self.inner.lock();
            match g.get_mut(id) {
                Some(h) if matches!(h.status, SideChannelStatus::Running) => {
                    h.cancel.cancel();
                    h.status = SideChannelStatus::Cancelled;
                    (Some(h.started_at_epoch_ms),)
                }
                _ => (None,),
            }
        };
        if let Some(started_epoch) = started_at_epoch_ms {
            let finished_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(started_epoch);
            let _ = self.events.send(SideChannelEvent::Cancelled {
                id: id.to_string(),
                finished_at_ms: finished_at,
            });
            true
        } else {
            false
        }
    }

    /// 将 side-channel 标记为完成（根据结果设为 `Done` 或 `Error`）。
    /// runner task 完成底层工作时调用。
    pub fn finish(&self, id: &str, status: SideChannelStatus, summary: Option<String>) {
        let (started_at_epoch_ms,) = {
            let mut g = self.inner.lock();
            match g.get_mut(id) {
                Some(h) => {
                    h.status = status;
                    (Some(h.started_at_epoch_ms),)
                }
                None => (None,),
            }
        };
        if let Some(_started_epoch) = started_at_epoch_ms {
            let finished_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let event = match status {
                SideChannelStatus::Done => SideChannelEvent::Done {
                    id: id.to_string(),
                    finished_at_ms,
                    summary,
                },
                SideChannelStatus::Error => SideChannelEvent::Error {
                    id: id.to_string(),
                    finished_at_ms,
                    message: summary.unwrap_or_default(),
                },
                SideChannelStatus::Cancelled => SideChannelEvent::Cancelled {
                    id: id.to_string(),
                    finished_at_ms,
                },
                SideChannelStatus::Running => SideChannelEvent::Done {
                    id: id.to_string(),
                    finished_at_ms,
                    summary,
                },
            };
            let _ = self.events.send(event);
        }
    }

    /// 列出所有 side-channel（快照）。按 started_at 升序排列，使
    /// UI 最先显示最早的项。
    pub fn list(&self) -> Vec<SideChannelInfo> {
        let g = self.inner.lock();
        let mut v: Vec<(SideChannelId, SideChannelInfo)> = g
            .iter()
            .map(|(id, h)| (id.clone(), h.snapshot(id)))
            .collect();
        v.sort_by(|a, b| a.1.started_at_ms.cmp(&b.1.started_at_ms));
        v.into_iter().map(|(_, info)| info).collect()
    }

    /// 按 id 查找单个 side-channel。
    pub fn get(&self, id: &str) -> Option<SideChannelInfo> {
        let g = self.inner.lock();
        g.get(id).map(|h| h.snapshot(id))
    }

    /// 当前正在运行的 side-channel 数量。
    pub fn running_count(&self) -> usize {
        self.inner
            .lock()
            .values()
            .filter(|h| matches!(h.status, SideChannelStatus::Running))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn reg() -> Arc<SideChannelRegistry> {
        Arc::new(SideChannelRegistry::new())
    }

    #[test]
    fn start_yields_id_and_cancel_token() {
        let r = reg();
        let (id, cancel) = r.start("default".into(), "hello".into());
        assert!(id.starts_with("side-"));
        assert_eq!(id.len(), 13); // “side-”加 8 位十六进制字符
        assert!(!cancel.is_cancelled());
        assert_eq!(r.list().len(), 1);
        assert_eq!(r.running_count(), 1);
    }

    #[test]
    fn cancel_marks_handles_and_emits_event() {
        let r = reg();
        let mut sub = r.subscribe_events();
        let (id, token) = r.start("default".into(), "x".into());
        assert!(r.cancel(&id), "should cancel running side-channel");
        // 取消 token 已触发。
        assert!(token.is_cancelled());
        let status = r.get(&id).unwrap().status;
        assert_eq!(status, "cancelled");
        assert_eq!(r.running_count(), 0);
        // 清空 channel：Started + Cancelled。
        let evt1 = sub.try_recv().expect("started event");
        assert!(matches!(evt1, SideChannelEvent::Started { .. }));
        let evt2 = sub.try_recv().expect("cancelled event");
        assert!(matches!(evt2, SideChannelEvent::Cancelled { .. }));
    }

    #[test]
    fn cancel_on_already_terminal_is_noop() {
        let r = reg();
        let (id, _) = r.start("default".into(), "x".into());
        r.finish(&id, SideChannelStatus::Done, Some("ok".into()));
        // 已处于终态；取消返回 false。
        assert!(!r.cancel(&id));
    }

    #[test]
    fn finish_done_emits_done_event() {
        let r = reg();
        let mut sub = r.subscribe_events();
        let (id, _) = r.start("default".into(), "hi".into());
        r.finish(&id, SideChannelStatus::Done, Some("bye".into()));
        // 清空 Started。
        let _ = sub.try_recv();
        let evt = sub.try_recv().expect("done event");
        match evt {
            SideChannelEvent::Done { summary, .. } => {
                assert_eq!(summary.as_deref(), Some("bye"));
            }
            _ => panic!("expected Done event"),
        }
    }

    #[test]
    fn list_snapshots_independent_cancels() {
        let r = reg();
        let (id1, t1) = r.start("default".into(), "x".into());
        let (id2, t2) = r.start("default".into(), "y".into());
        assert_ne!(id1, id2);
        // 取消 id1 不得影响 id2。
        r.cancel(&id1);
        assert!(t1.is_cancelled());
        assert!(!t2.is_cancelled());
    }

    #[test]
    fn cancel_does_not_block_future_starts() {
        let r = reg();
        // 稍作压力测试，确认 Registry 不会因已取消条目而永久
        // 卡住（冒烟检查；不涉及正确性泄漏）。
        for i in 0..20 {
            let (id, _) = r.start("default".into(), format!("p-{i}"));
            r.cancel(&id);
        }
        assert_eq!(r.running_count(), 0);
        // 短暂休眠后，Registry 仍可向新订阅者发送 event。
        std::thread::sleep(Duration::from_millis(1));
        let mut sub = r.subscribe_events();
        let _ = r.start("default".into(), "fresh".into());
        let evt = sub.try_recv().expect("started event");
        assert!(matches!(evt, SideChannelEvent::Started { .. }));
    }
}
