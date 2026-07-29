//! Remote mode config (Phase 2 item 2: Tailscale + iOS daemon).
//!
//! Mirrors CodexMonitor's `RemoteBackendSettings` shape but lives in `state/`
//! (vendor is read-only). Process-in-memory only — restart drops config;
//! persistence (keyring or `~/.reflect/remote.toml`) is a follow-up.

use serde::{Deserialize, Serialize};

/// User-editable remote daemon target. Field names are snake_case (matches the
/// Rust defaults — the IPC layer relies on the same shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RemoteConfig {
    /// `host:port` of the remote daemon (iOS or another machine).
    pub host: String,
    /// Optional port override; defaults to 4732 (the desktop daemon's
    /// `DEFAULT_DAEMON_PORT` constant in `app-core::tailscale`).
    pub port: u16,
    /// Shared bearer token for the line-delimited JSON-RPC link.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Whether to auto-connect on launch. Currently advisory; the actual
    /// "connect on launch" hook is wired in a follow-up.
    #[serde(default)]
    pub auto_connect: bool,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 4732,
            auth_token: None,
            auto_connect: false,
        }
    }
}

impl RemoteConfig {
    /// Render as `host:port` (matches what the desktop daemon's TCP
    /// transport expects).
    pub fn endpoint(&self) -> String {
        if self.host.contains(':') {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// True when both host and auth_token are populated (minimum viable
    /// connection).
    pub fn is_ready(&self) -> bool {
        !self.host.is_empty() && self.auth_token.as_deref().is_some_and(|t| !t.is_empty())
    }
}

/// Runtime status of the desktop side (transport up/down, last error…).
/// Mirrors the read-side of CodexMonitor's `RemoteBackendStatus`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    /// `connected` | `disconnected` | `error`.
    pub state: String,
    /// Free-form diagnostic message (last error / connection note).
    pub message: Option<String>,
    /// When `state == connected`, the host we are connected to.
    pub endpoint: Option<String>,
    /// Unix epoch milliseconds when the current state was entered.
    pub since_ms: Option<i64>,
}

impl RemoteStatus {
    pub fn disconnected(message: impl Into<String>) -> Self {
        Self {
            state: "disconnected".into(),
            message: Some(message.into()),
            endpoint: None,
            since_ms: Some(now_epoch_ms()),
        }
    }
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_endpoint_uses_default_port() {
        let cfg = RemoteConfig {
            host: "node.tail.net".into(),
            ..Default::default()
        };
        assert_eq!(cfg.endpoint(), "node.tail.net:4732");
    }

    #[test]
    fn endpoint_preserves_explicit_port() {
        let cfg = RemoteConfig {
            host: "node.tail.net:5555".into(),
            ..Default::default()
        };
        assert_eq!(cfg.endpoint(), "node.tail.net:5555");
    }

    #[test]
    fn is_ready_requires_both_host_and_token() {
        let mut cfg = RemoteConfig::default();
        assert!(!cfg.is_ready(), "empty config not ready");
        cfg.host = "host".into();
        assert!(!cfg.is_ready(), "missing token");
        cfg.auth_token = Some("t".into());
        assert!(cfg.is_ready(), "both host + token ready");
    }

    #[test]
    fn disconnected_status_carries_since_ms() {
        let s = RemoteStatus::disconnected("no peer");
        assert_eq!(s.state, "disconnected");
        assert_eq!(s.message.as_deref(), Some("no peer"));
        assert!(s.since_ms.is_some());
    }
}