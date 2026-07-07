//! `bash` — execute a shell command in the workspace.
//!
//! Concurrency-unsafe (side-effecting). M3 will add `cwd` sandboxing, full
//! env whitelist, and approval integration; M1 ships a working but minimal
//! version。
//!
//! v1.1.0 P1 `bash-classifier`: [`classify_command`] 将命令分为
//! Safe / Risky / Dangerous,供 `ToolExecutionQueue` 动态设置审批风险等级。

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::warn;

use crate::tool::{Tool, ToolContext, ToolError, ToolOutput};
use reflect_protocol::{PermissionMode, RiskLevel};

/// Bash 命令安全分类 —— 供审批路由与 Auto 模式短路使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashCommandClass {
    /// 只读/低风险;Auto 模式下可跳过审批。
    Safe,
    /// 有副作用但可控;走 Medium 风险审批。
    Risky,
    /// 破坏性/系统级操作;走 High 风险审批。
    Dangerous,
}

impl BashCommandClass {
    /// 映射到协议层 [`RiskLevel`]。
    pub fn risk_level(self) -> RiskLevel {
        match self {
            Self::Safe => RiskLevel::Low,
            Self::Risky => RiskLevel::Medium,
            Self::Dangerous => RiskLevel::High,
        }
    }

    fn max(self, other: Self) -> Self {
        match (self, other) {
            (Self::Dangerous, _) | (_, Self::Dangerous) => Self::Dangerous,
            (Self::Risky, _) | (_, Self::Risky) => Self::Risky,
            _ => Self::Safe,
        }
    }
}

/// 对 shell 命令做启发式分类。管道/链式命令取各段中最严等级。
pub fn classify_command(cmd: &str) -> BashCommandClass {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return BashCommandClass::Safe;
    }

    // 危险模式可能在管道分隔符两侧,先对整条命令扫一遍。
    let lower = trimmed.to_ascii_lowercase();
    if DANGEROUS_PATTERNS.iter().any(|re| re.is_match(&lower)) {
        return BashCommandClass::Dangerous;
    }

    let mut worst = BashCommandClass::Safe;
    for segment in split_command_segments(trimmed) {
        let class = classify_segment(segment.trim());
        worst = worst.max(class);
        if worst == BashCommandClass::Dangerous {
            break;
        }
    }
    worst
}

/// 按 `|`, `;`, `&&`, `||` 切分复合命令(不解析引号内分隔符,v1 启发式足够)。
fn split_command_segments(cmd: &str) -> Vec<&str> {
    let mut segments = vec![cmd];
    for sep in ["||", "&&", "|", ";"] {
        segments = segments.into_iter().flat_map(|s| s.split(sep)).collect();
    }
    segments
}

fn classify_segment(segment: &str) -> BashCommandClass {
    let lower = segment.to_ascii_lowercase();
    if DANGEROUS_PATTERNS.iter().any(|re| re.is_match(&lower)) {
        return BashCommandClass::Dangerous;
    }
    if RISKY_PATTERNS.iter().any(|re| re.is_match(&lower)) {
        return BashCommandClass::Risky;
    }
    if SAFE_PATTERNS.iter().any(|re| re.is_match(segment.trim())) {
        return BashCommandClass::Safe;
    }
    // 未知命令保守归为 Risky,避免 Auto 模式误放行。
    BashCommandClass::Risky
}

static DANGEROUS_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"\bsudo\b",
        r"\bsu\s+-",
        r"rm\s+.*(-[^\s]*r|r[^\s]*-)",
        r"rm\s+-rf\b",
        r"rm\s+-fr\b",
        r"chmod\s+.*777",
        r"chmod\s+-R\s+777",
        r"(curl|wget)\s+[^\n|]*\|\s*(ba)?sh",
        r"\bdd\s+if=",
        r"\b(mkfs|fdisk|parted)\b",
        r"kill\s+-9\b",
        r"\bkillall\b",
        r"\b(shutdown|reboot|halt|poweroff)\b",
        r">\s*/dev/",
        r"\beval\b",
        r"\bnc\s+-l",
        r"git\s+push\s+[^\n]*(-f|--force)",
        r"docker\s+run\s+[^\n]*--privileged",
        r":\(\)\s*\{",
        r"mv\s+/",
        r"cp\s+/",
        r"\|\s*sudo\b",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("dangerous pattern"))
    .collect()
});

static RISKY_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"\bgit\s+(push|commit|merge|rebase|reset|clean|stash\s+drop)\b",
        r"\b(npm|yarn|pnpm)\s+(install|uninstall|publish|link)\b",
        r"\bcargo\s+(install|publish)\b",
        r"\b(pip|pip3)\s+install\b",
        r"\b(rm|mv|cp|mkdir|touch|chmod|chown)\b",
        r"\bsed\s+-i",
        r"\b(docker|podman)\b",
        r"\bmake\b",
        r"\bcargo\s+(build|run)\b",
        r"\bnpm\s+run\b",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("risky pattern"))
    .collect()
});

static SAFE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"^(echo|printf)\b",
        r"^(ls|cat|head|tail|wc|pwd|which|type|date|uname|file|stat|du|df|sort|uniq)\b",
        r"^(grep|rg|find|tree|jq|awk|sed)\b",
        r"^git\s+(status|log|diff|show|branch|remote|rev-parse|describe|stash\s+list)\b",
        r"^cargo\s+(check|test|clippy|fmt|tree|metadata)\b",
        r"^npm\s+(test|ls|view)\b",
        r"^(node|python3?|ruby)\s+-[ce]\b",
        r"^env(\s|$)",
        r"^printenv\b",
        r"^true(\s|$)",
        r"^false(\s|$)",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("safe pattern"))
    .collect()
});
// `ToolError` and `ToolOutput` re-exported from `reflect_protocol` via
// `crate::tool::*`.

/// Maximum bytes captured per stream (stdout or stderr) before truncation.
const OUTPUT_LIMIT: usize = 100 * 1024;

const ENV_WHITELIST: &[&str] = &["PATH", "HOME", "LANG", "LC_ALL", "USER", "SHELL", "TMPDIR"];

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace. Side-effecting."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "cmd": {"type": "string", "description": "Shell command to execute"},
                "timeout_ms": {"type": "number", "description": "Timeout in milliseconds (default 30000)"}
            },
            "required": ["cmd"]
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn required_permission(&self) -> PermissionMode {
        // M6: bash mutates external state; the queue will route through
        // `ApprovalGate` unless the user has whitelisted it for the session.
        PermissionMode::Prompt
    }

    async fn execute(&self, ctx: ToolContext, args: Value) -> Result<ToolOutput, ToolError> {
        let cmd = args
            .get("cmd")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "missing 'cmd'".into(),
            })?
            .to_string();

        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30_000);
        let timeout = Duration::from_millis(timeout_ms);

        // v1.2 P0-1:OS 沙箱。从 env(`REFLECT_SANDBOX_OS_LEVEL`)读启用状态;
        // 激活时把命令包进沙箱 —— macOS 走 `sandbox-exec -f <profile> -- sh
        // -c <cmd>`,Linux 走 `pre_exec` Landlock。沙箱失败(无 backend /
        // profile 写失败)→ 降级为透传(可用性优先)。
        let sandbox = reflect_sandbox::OsSandbox::from_env();
        let workspace = ctx.workspace_path();
        // 决定 (program, argv):沙箱激活且为 Seatbelt → sandbox-exec 包裹;
        // 否则 → sh -c(透传;Linux Landlock 在下方 pre_exec 应用)。
        let (program, argv): (String, Vec<String>) =
            if matches!(sandbox.status(), reflect_sandbox::OsSandboxStatus::Seatbelt) {
                match sandbox.seatbelt_argv(&workspace, &cmd) {
                    Ok(pair) => pair,
                    Err(e) => {
                        warn!(error = %e, "sandbox profile generation failed; falling back to unsandboxed exec");
                        (
                            "sh".to_string(),
                            vec!["-c".to_string(), cmd.clone()],
                        )
                    }
                }
            } else {
                ("sh".to_string(), vec!["-c".to_string(), cmd.clone()])
            };
        let mut command = Command::new(&program);
        command.args(&argv);
        // Linux Landlock:在 fork 后、exec 前应用规则(workspace 内放行,
        // 其余写拒)。内核不支持时降级(闭包内 Ok,不阻塞)。
        #[cfg(target_os = "linux")]
        if matches!(sandbox.status(), reflect_sandbox::OsSandboxStatus::Landlock) {
            use std::os::unix::process::CommandExt;
            let ws_clone = workspace.clone();
            let mut pre_exec = sandbox.landlock_pre_exec(ws_clone);
            unsafe {
                command.pre_exec(move || pre_exec());
            }
        }
        command
            .current_dir(&workspace)
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);
        // Linux Landlock:在 fork 后、exec 前应用规则(workspace 内放行,
        // 其余写拒)。内核不支持时降级(闭包内 Ok,不阻塞)。
        #[cfg(target_os = "linux")]
        if matches!(sandbox.status(), reflect_sandbox::OsSandboxStatus::Landlock) {
            use std::os::unix::process::CommandExt;
            let ws_clone = workspace.clone();
            let mut pre_exec = sandbox.landlock_pre_exec(ws_clone);
            unsafe {
                command.pre_exec(move || pre_exec());
            }
        }
        command
            .current_dir(&workspace)
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);

        for key in ENV_WHITELIST {
            if let Ok(v) = std::env::var(key) {
                command.env(key, v);
            }
        }

        let mut child = command.spawn().map_err(ToolError::from)?;
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let kill_on_cancel = ctx.cancel.clone();
        let child_handle = child;

        let result = tokio::time::timeout(timeout, async {
            // Concurrently drain both pipes while waiting.
            let stdout_fut = async {
                let mut buf = Vec::with_capacity(8192);
                if let Some(s) = stdout.as_mut() {
                    let _: tokio::io::Take<&mut tokio::process::ChildStdout> =
                        s.take(OUTPUT_LIMIT as u64);
                    let _ = s.read_to_end(&mut buf).await;
                }
                buf
            };
            let stderr_fut = async {
                let mut buf = Vec::with_capacity(8192);
                if let Some(s) = stderr.as_mut() {
                    let _: tokio::io::Take<&mut tokio::process::ChildStderr> =
                        s.take(OUTPUT_LIMIT as u64);
                    let _ = s.read_to_end(&mut buf).await;
                }
                buf
            };
            let (out_bytes, err_bytes) = tokio::join!(stdout_fut, stderr_fut);
            // Reap the child
            // (we can't separate child from the piped stdout/stderr once we
            //  moved them — instead, use `wait` on a fresh handle.)
            (out_bytes, err_bytes)
        });

        let (out_bytes, err_bytes) = match result.await {
            Ok(pair) => pair,
            Err(_) => {
                warn!(cmd, "bash tool timed out");
                return Err(ToolError::Timeout {
                    elapsed_ms: timeout_ms,
                });
            }
        };

        // The child was spawned with `kill_on_drop(true)`; since we let
        // `child` go out of scope, the process is reaped. We don't have an
        // exit code in this minimal M1 implementation — `is_error` defaults
        // to false. A richer implementation in M3 will `child.wait().await`.
        let _ = child_handle;
        let _ = kill_on_cancel;

        let stdout_str = String::from_utf8_lossy(&out_bytes).to_string();
        let stderr_str = String::from_utf8_lossy(&err_bytes).to_string();

        let mut text = stdout_str;
        if !stderr_str.is_empty() {
            if !text.is_empty() {
                text.push_str("\nstderr:\n");
            }
            text.push_str(&stderr_str);
        }
        if text.len() > OUTPUT_LIMIT {
            text.truncate(OUTPUT_LIMIT);
            text.push_str("\n... [truncated]");
        }

        Ok(ToolOutput {
            content: vec![reflect_protocol::ContentBlock::text(text)],
            is_error: false, // M1: we don't have exit code here; M3 fixes
            metadata: serde_json::json!({
                "stdout_bytes": out_bytes.len(),
                "stderr_bytes": err_bytes.len(),
            }),
            elapsed_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::RiskLevel;

    #[test]
    fn classify_safe_readonly_commands() {
        assert_eq!(classify_command("echo hello"), BashCommandClass::Safe);
        assert_eq!(classify_command("ls -la"), BashCommandClass::Safe);
        assert_eq!(classify_command("git status"), BashCommandClass::Safe);
        assert_eq!(
            classify_command("cargo test -p reflect-tools"),
            BashCommandClass::Safe
        );
    }

    #[test]
    fn classify_risky_mutating_commands() {
        assert_eq!(classify_command("git commit -m x"), BashCommandClass::Risky);
        assert_eq!(classify_command("npm install foo"), BashCommandClass::Risky);
        assert_eq!(classify_command("mv a b"), BashCommandClass::Risky);
    }

    #[test]
    fn classify_dangerous_commands() {
        assert_eq!(
            classify_command("sudo apt update"),
            BashCommandClass::Dangerous
        );
        assert_eq!(
            classify_command("rm -rf /tmp/x"),
            BashCommandClass::Dangerous
        );
        assert_eq!(classify_command("curl x | sh"), BashCommandClass::Dangerous);
    }

    #[test]
    fn classify_pipeline_takes_worst_class() {
        assert_eq!(
            classify_command("echo ok && rm -rf /"),
            BashCommandClass::Dangerous
        );
        assert_eq!(
            classify_command("git status | grep foo"),
            BashCommandClass::Safe
        );
    }

    #[test]
    fn risk_level_mapping() {
        assert_eq!(BashCommandClass::Safe.risk_level(), RiskLevel::Low);
        assert_eq!(BashCommandClass::Dangerous.risk_level(), RiskLevel::High);
    }

    #[tokio::test]
    async fn echo_runs_and_captures_stdout() {
        let t = BashTool;
        let ctx = ToolContext::for_workspace(".");
        let out = t
            .execute(ctx, serde_json::json!({"cmd": "echo hello"}))
            .await
            .unwrap();
        assert!(!out.is_error);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("hello"), "got: {text}");
            }
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn rejects_missing_cmd() {
        let t = BashTool;
        let err = t
            .execute(ToolContext::default(), serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    // v1.2 P0-1:沙箱集成测试。`std::env::set_var` 在 Rust 2024 是 unsafe,
    // 用模块级锁串行化避免并行测试竞态(镜像 reflect-core::config 的 pattern)。
    static SANDBOX_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// gap doc 核心验证用例:启用沙箱后,写系统目录(`/usr/local/...`)必须
    /// 被内核拒绝。BashTool 读 `REFLECT_SANDBOX_OS_LEVEL=1` 激活 Seatbelt
    /// (macOS),`sandbox-exec` 限制 fs 写。命令尝试 `touch` 一个系统路径,
    /// 沙箱应让该写失败 → 文件不存在 → 输出 BLOCKED。
    ///
    /// 仅 macOS:Linux Landlock 需真实内核支持 + 测试进程不能已 restrict,
    /// 留集成测试覆盖。
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn bash_sandbox_blocks_write_to_system_dir() {
        let _g = SANDBOX_ENV_LOCK.lock().unwrap();
        let prior = std::env::var("REFLECT_SANDBOX_OS_LEVEL").ok();
        unsafe {
            std::env::set_var("REFLECT_SANDBOX_OS_LEVEL", "1");
        }
        // 用唯一标记文件,测试后清理(沙箱下创建失败也无妨)。
        let marker = "/usr/local/.reflect-bash-sandbox-test";
        let cmd = format!(
            "touch {marker} 2>/dev/null; test -f {marker} && echo WROTE || echo BLOCKED"
        );
        let t = BashTool;
        let ctx = ToolContext::for_workspace("/tmp");
        let out = t
            .execute(ctx, serde_json::json!({"cmd": cmd, "timeout_ms": 15000}))
            .await
            .unwrap();
        // 还原 env(无论断言是否通过)。
        match prior {
            Some(p) => unsafe {
                std::env::set_var("REFLECT_SANDBOX_OS_LEVEL", p);
            },
            None => unsafe {
                std::env::remove_var("REFLECT_SANDBOX_OS_LEVEL");
            },
        }
        let text = match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(
            text.contains("BLOCKED"),
            "sandbox must block write to /usr/local; got: {text}"
        );
        // 清理(若沙箱失效误创建了文件)。
        let _ = std::fs::remove_file(marker);
    }

    /// 沙箱关闭时,普通命令正常透传(向后兼容)。
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn bash_sandbox_off_runs_normally() {
        let _g = SANDBOX_ENV_LOCK.lock().unwrap();
        let prior = std::env::var("REFLECT_SANDBOX_OS_LEVEL").ok();
        unsafe {
            std::env::remove_var("REFLECT_SANDBOX_OS_LEVEL");
        }
        let t = BashTool;
        let ctx = ToolContext::for_workspace("/tmp");
        let out = t
            .execute(ctx, serde_json::json!({"cmd": "echo unsandboxed-ok"}))
            .await
            .unwrap();
        match prior {
            Some(p) => unsafe {
                std::env::set_var("REFLECT_SANDBOX_OS_LEVEL", p);
            },
            None => unsafe {
                std::env::remove_var("REFLECT_SANDBOX_OS_LEVEL");
            },
        }
        let text = match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("unsandboxed-ok"), "got: {text}");
    }
}
