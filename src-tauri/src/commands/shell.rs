//! B8-01: terminal shell exec —— 流式 stdout/stderr 到前端事件。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// Shell session metadata returned to the frontend on `reflect_run_shell`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellSession {
    /// Unique session id (UUID v4); used to correlate subsequent
    /// `reflect_terminal_output` events and to call `reflect_kill_shell`.
    pub id: String,
    /// Command that was launched.
    pub command: String,
    /// Working directory (workspace at launch time).
    pub cwd: String,
}

/// Payload emitted on `reflect_terminal_output` event.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShellOutputChunk {
    pub session_id: String,
    /// "stdout" | "stderr" | "exit" | "error".
    pub stream: String,
    /// For stdout/stderr: utf-8 decoded line; for exit: exit code as string.
    pub data: String,
    /// Monotonic sequence within this session (for ordering).
    pub seq: u64,
}

/// Spawn `cmd` (single shell expression), stream stdout/stderr as
/// `reflect_terminal_output` events tagged with `session_id`. Returns the
/// session id so the frontend can match chunks.
///
/// Real PTY (resize / raw mode) is out of scope for the first cut — we use
/// `tokio::process::Command` with piped stdout/stderr and emit line chunks.
/// The frontend renders them with ANSI-safe escaping.
#[tauri::command]
pub async fn reflect_run_shell(
    agent: State<'_, MinimalAgent>,
    app: AppHandle,
    cmd: String,
) -> CommandResult<ShellSession> {
    let cwd = agent.workspace().display().to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let workdir = agent.workspace().clone();

    // Build command. Use shell to support pipes/redirects (zsh on macOS, sh elsewhere).
    let shell = if cfg!(target_os = "macos") {
        "/bin/zsh"
    } else {
        "/bin/sh"
    };
    let mut command = TokioCommand::new(shell);
    command
        .arg("-lc")
        .arg(&cmd)
        .current_dir(&workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let mut child = command.spawn().map_err(|e| CommandError {
        msg: format!("spawn failed: {e}"),
    })?;

    let stdout = child.stdout.take().ok_or_else(|| CommandError {
        msg: "no stdout handle".into(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| CommandError {
        msg: "no stderr handle".into(),
    })?;

    // Register kill handle BEFORE spawning readers so kill can race safely.
    let child_lock = Arc::new(tokio::sync::Mutex::new(Some(child)));
    agent.register_shell_session(id.clone(), child_lock.clone());

    // stdout reader — also waits for process exit and emits an "exit" event.
    {
        let app = app.clone();
        let id = id.clone();
        let lock = child_lock.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            let mut seq: u64 = 0;
            while let Ok(Some(line)) = reader.next_line().await {
                seq += 1;
                let _ = app.emit(
                    "reflect_terminal_output",
                    ShellOutputChunk {
                        session_id: id.clone(),
                        stream: "stdout".into(),
                        data: line,
                        seq,
                    },
                );
            }
            // Wait for exit (if process hasn't been killed yet).
            let mut g = lock.lock().await;
            if let Some(mut c) = g.take() {
                match c.wait().await {
                    Ok(status) => {
                        let _ = app.emit(
                            "reflect_terminal_output",
                            ShellOutputChunk {
                                session_id: id.clone(),
                                stream: "exit".into(),
                                data: status.code().map(|c| c.to_string()).unwrap_or_default(),
                                seq: seq.saturating_add(1),
                            },
                        );
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "reflect_terminal_output",
                            ShellOutputChunk {
                                session_id: id.clone(),
                                stream: "error".into(),
                                data: e.to_string(),
                                seq: seq.saturating_add(1),
                            },
                        );
                    }
                }
            }
        });
    }

    // stderr reader
    {
        let app = app.clone();
        let id = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            let mut seq: u64 = 1_000_000; // distinct namespace
            while let Ok(Some(line)) = reader.next_line().await {
                seq += 1;
                let _ = app.emit(
                    "reflect_terminal_output",
                    ShellOutputChunk {
                        session_id: id.clone(),
                        stream: "stderr".into(),
                        data: line,
                        seq,
                    },
                );
            }
        });
    }

    Ok(ShellSession {
        id,
        command: cmd,
        cwd,
    })
}

/// Kill a running shell session by id. Idempotent: missing sessions are not an error.
#[tauri::command]
pub async fn reflect_kill_shell(
    agent: State<'_, MinimalAgent>,
    session_id: String,
) -> CommandResult<()> {
    if let Some(handle) = agent.take_shell_session(&session_id) {
        let mut g = handle.lock().await;
        if let Some(mut child) = g.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
    Ok(())
}

/// List active shell session ids (diagnostic).
#[tauri::command]
pub async fn reflect_list_shell_sessions(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<String>> {
    Ok(agent.list_shell_sessions())
}
