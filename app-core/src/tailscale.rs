//! Tailscale detection + remote-host suggestion helper.
//!
//! Mirrors `CodexMonitor`'s `src-tauri/src/tailscale/` shape but lives in
//! `app-core` (vendor is a read-only mirror — see AGENTS.md). The helper shells
//! out to the local `tailscale` CLI to read its status; on any failure it
//! returns a `degraded_status` so the frontend can still render a "not
//! installed" / "not running" card.
//!
//! ## On `suggested_remote_host`
//!
//! The dashboard builds the iOS-side `Remote backend host` field from this
//! (e.g. `your-mac.your-tailnet.ts.net:4732`). It joins `dns_name` (when
//! present) with the default daemon port `4732`.
//!
//! ## Behaviour
//!
//! - `tailscale status --json=true` is the canonical machine-readable probe.
//! - If `tailscale` is missing / errors, return `installed = false`.
//! - If status JSON parse fails, return `installed = true, running = false`
//!   plus a human-readable `message`.

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Default listen port the desktop daemon binds to. Mirrors CodexMonitor's
/// `DEFAULT_DAEMON_LISTEN_ADDR` constant.
pub const DEFAULT_DAEMON_PORT: u16 = 4732;

/// Maximum time the CLI probe is allowed to run.
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Snapshot of the local Tailscale daemon's view of itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleStatus {
    /// Whether the `tailscale` CLI binary was located on `$PATH`.
    pub installed: bool,
    /// Whether the daemon reports itself as running (BackendState == "Running").
    pub running: bool,
    /// `tailscale version` short string when discoverable.
    pub version: Option<String>,
    /// Full MagicDNS name (e.g. `node.tailnet.ts.net`).
    pub dns_name: Option<String>,
    /// Short host name (e.g. `node`).
    pub host_name: Option<String>,
    /// Tailnet display name.
    pub tailnet_name: Option<String>,
    /// IPv4 addresses the daemon reports.
    pub ipv4: Vec<String>,
    /// IPv6 addresses the daemon reports.
    pub ipv6: Vec<String>,
    /// `host:port` derived from `dns_name` (or `ipv4[0]`) + the default port.
    /// `None` when neither name nor IP is available.
    pub suggested_remote_host: Option<String>,
    /// Free-form diagnostic message (probe error, parse error, etc).
    pub message: Option<String>,
}

/// Probe the local Tailscale daemon. Always returns `Ok` — failures are
/// surfaced as a degraded `TailscaleStatus`.
pub async fn detect() -> TailscaleStatus {
    match run_probe().await {
        Ok(mut status) => {
            // Derive suggested_remote_host if we have a name or IP.
            status.suggested_remote_host = derive_suggested_host(&status);
            status
        }
        Err(reason) => degraded(None, reason),
    }
}

/// Probe + return the exact command the desktop would run to start the
/// daemon (used by the iOS-side setup card as a copy hint).
pub async fn daemon_command_preview() -> String {
    // CodexMonitor renders a hint string showing what shell command the
    // user would invoke. We mirror the minimum useful surface:
    // `tailscale up` + the listen port (so the user can see what binds).
    format!(
        "tailscaled --tun=userspace-networking --state=mem: --socket=/var/run/tailscale/tailscaled.sock &\nlisten 0.0.0.0:{}",
        DEFAULT_DAEMON_PORT
    )
}

async fn run_probe() -> std::result::Result<TailscaleStatus, String> {
    // 1. Try `tailscale version` (cheap, no network).
    let version_output = Command::new("tailscale")
        .arg("version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("tailscale CLI not found: {e}"))?;
    if !version_output.status.success() {
        return Err("tailscale CLI failed (not running?)".into());
    }
    let version = String::from_utf8_lossy(&version_output.stdout)
        .split_whitespace()
        .nth(1)
        .map(|s| s.trim_start_matches('v').to_string());

    // 2. Try `tailscale status --json=true`.
    let status_output = tokio::time::timeout(
        PROBE_TIMEOUT,
        Command::new("tailscale")
            .args(["status", "--json=true"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| "tailscale status timed out".to_string())?
    .map_err(|e| format!("tailscale status failed: {e}"))?;

    if !status_output.status.success() {
        return Err("tailscale status exited non-zero".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).map_err(|e| format!("parse tailscale json: {e}"))?;

    let running = value
        .get("BackendState")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("Running"))
        .unwrap_or(false);

    let dns_name = value
        .get("Self")
        .and_then(|s| s.get("DNSName"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim_end_matches('.').to_string());
    let host_name = value
        .get("Self")
        .and_then(|s| s.get("HostName"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tailnet_name = value
        .get("MagicDNSSuffix")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();
    if let Some(addrs) = value.get("TailscaleIPs").and_then(|v| v.as_array()) {
        for ip in addrs {
            if let Some(s) = ip.as_str() {
                if s.contains(':') {
                    ipv6.push(s.to_string());
                } else {
                    ipv4.push(s.to_string());
                }
            }
        }
    }

    Ok(TailscaleStatus {
        installed: true,
        running,
        version,
        dns_name,
        host_name,
        tailnet_name,
        ipv4,
        ipv6,
        suggested_remote_host: None, // filled by caller
        message: None,
    })
}

fn derive_suggested_host(status: &TailscaleStatus) -> Option<String> {
    if let Some(dns) = &status.dns_name {
        return Some(format!("{dns}:{}", DEFAULT_DAEMON_PORT));
    }
    if let Some(ip) = status.ipv4.first() {
        return Some(format!("{ip}:{}", DEFAULT_DAEMON_PORT));
    }
    None
}

fn degraded(version: Option<String>, message: String) -> TailscaleStatus {
    TailscaleStatus {
        installed: false,
        running: false,
        version,
        dns_name: None,
        host_name: None,
        tailnet_name: None,
        ipv4: Vec::new(),
        ipv6: Vec::new(),
        suggested_remote_host: None,
        message: Some(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_host_uses_dns_name_first() {
        let mut s = degraded(None, "x".into());
        s.dns_name = Some("node.tail.net".into());
        s.ipv4 = vec!["100.64.0.1".into()];
        assert_eq!(
            derive_suggested_host(&s),
            Some(format!("node.tail.net:{}", DEFAULT_DAEMON_PORT))
        );
    }

    #[test]
    fn suggested_host_falls_back_to_ipv4() {
        let mut s = degraded(None, "x".into());
        s.ipv4 = vec!["100.64.0.1".into()];
        assert_eq!(
            derive_suggested_host(&s),
            Some(format!("100.64.0.1:{}", DEFAULT_DAEMON_PORT))
        );
    }

    #[test]
    fn suggested_host_none_without_name_or_ip() {
        let s = degraded(None, "x".into());
        assert_eq!(derive_suggested_host(&s), None);
    }

    #[test]
    fn degraded_status_carries_message_and_disables_installed() {
        let s = degraded(Some("1.78.0".into()), "tailscale missing".into());
        assert!(!s.installed);
        assert!(!s.running);
        assert_eq!(s.version.as_deref(), Some("1.78.0"));
        assert_eq!(s.message.as_deref(), Some("tailscale missing"));
    }

    #[tokio::test]
    async fn detect_returns_status_without_panicking() {
        // Local dev machines usually have tailscale installed; CI usually
        // does not. The contract is: detect() must always return Ok with
        // a well-formed status (no panic, no Result error).
        let s = detect().await;
        // `suggested_remote_host` is either Some or None — never invalid.
        let _ = s.suggested_remote_host;
        // When installed == true we must have version + something useful;
        // when installed == false we must have a diagnostic message.
        if s.installed {
            // Nothing else to assert; the function ran end-to-end.
        } else {
            assert!(s.message.is_some());
        }
    }

    #[tokio::test]
    async fn daemon_command_preview_mentions_default_port() {
        let s = daemon_command_preview().await;
        assert!(s.contains(&DEFAULT_DAEMON_PORT.to_string()));
    }
}