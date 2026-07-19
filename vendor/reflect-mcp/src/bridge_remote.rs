//! Bridge 远程控制 stub —— 远程 TUI/agent 桥接占位。

/// Bridge 状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeRemoteStatus {
    Disconnected,
    StubConfigured,
}

/// Bridge 远程 stub。
#[derive(Debug, Clone, Default)]
pub struct BridgeRemote {
    pub endpoint: Option<String>,
}

impl BridgeRemote {
    pub fn status(&self) -> BridgeRemoteStatus {
        if self.endpoint.as_ref().is_some_and(|s| !s.is_empty()) {
            BridgeRemoteStatus::StubConfigured
        } else {
            BridgeRemoteStatus::Disconnected
        }
    }

    pub fn status_line(&self) -> String {
        match self.status() {
            BridgeRemoteStatus::Disconnected => {
                "bridge-remote: stub — 配置 [bridge].endpoint 后可用(v2.x)".into()
            }
            BridgeRemoteStatus::StubConfigured => {
                format!("bridge-remote: stub — endpoint={:?}", self.endpoint)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disconnected() {
        assert_eq!(
            BridgeRemote::default().status(),
            BridgeRemoteStatus::Disconnected
        );
    }
}
