//! Shell 会话管理 —— 跟踪运行中的终端进程,提供注册/移除/列举接口。
//!
//! 每个 shell 会话通过唯一 id 索引,持有 `tokio::process::Child` 的句柄
//! 用于后续 kill 操作。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::process::Child;

/// Shell 进程句柄包装。
pub(crate) type ShellProcess = Arc<tokio::sync::Mutex<Option<Child>>>;

#[derive(Default)]
pub(crate) struct ShellSessions {
    processes: Mutex<HashMap<String, ShellProcess>>,
}

impl ShellSessions {
    pub(crate) fn register(&self, id: String, child: ShellProcess) {
        self.processes.lock().insert(id, child);
    }

    pub(crate) fn take(&self, id: &str) -> Option<ShellProcess> {
        self.processes.lock().remove(id)
    }

    pub(crate) fn list(&self) -> Vec<String> {
        self.processes.lock().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_registers_lists_and_takes_sessions() {
        let sessions = ShellSessions::default();
        let process = Arc::new(tokio::sync::Mutex::new(None));
        sessions.register("session-1".into(), process.clone());

        assert_eq!(sessions.list(), vec!["session-1"]);
        assert!(Arc::ptr_eq(&sessions.take("session-1").unwrap(), &process));
        assert!(sessions.list().is_empty());
        assert!(sessions.take("missing").is_none());
    }
}
