//! B6: Git integration —— status / diff / log。

use std::path::PathBuf;
use std::process::Command as StdCommand;

use serde::{Deserialize, Serialize};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: String, // "M" | "A" | "D" | "??" | "R"
    pub old_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<GitStatusEntry>,
    pub raw: String,
    pub is_repo: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub timestamp: i64,
    pub subject: String,
}

fn git_in_workspace() -> Option<PathBuf> {
    if let Some(p) = MinimalAgent::workspace_override() {
        return Some(p);
    }
    std::env::current_dir().ok()
}

fn run_git(args: &[&str]) -> Result<std::process::Output, CommandError> {
    let cwd = git_in_workspace().ok_or_else(|| CommandError {
        msg: "no workspace".into(),
    })?;
    StdCommand::new("git")
        .current_dir(&cwd)
        .args(args)
        .output()
        .map_err(|e| CommandError {
            msg: format!("spawn git: {e}"),
        })
}

/// `git status --porcelain=v1 --branch` → structured status.
#[tauri::command]
pub async fn reflect_git_status() -> CommandResult<GitStatus> {
    // First, is this even a git repo?
    let probe = run_git(&["rev-parse", "--is-inside-work-tree"]).map_err(CommandError::from)?;
    let is_repo = String::from_utf8_lossy(&probe.stdout).trim() == "true";
    if !is_repo {
        return Ok(GitStatus {
            branch: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            entries: Vec::new(),
            raw: String::new(),
            is_repo: false,
        });
    }
    let out = run_git(&["status", "--porcelain=v1", "--branch"]).map_err(CommandError::from)?;
    if !out.status.success() {
        return Err(CommandError {
            msg: format!(
                "git status failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
        });
    }
    let raw = String::from_utf8_lossy(&out.stdout).to_string();
    let mut branch: Option<String> = None;
    let mut upstream: Option<String> = None;
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut entries: Vec<GitStatusEntry> = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // Branch header: "## <branch>" or "## <branch>...<upstream> [ahead N, behind M]"
            if let Some((b, after)) = rest.split_once("...") {
                branch = Some(b.to_string());
                if let Some((up, ab)) = after.split_once(' ') {
                    upstream = Some(up.to_string());
                    let ab = ab.trim_start_matches('[').trim_end_matches(']');
                    for part in ab.split(", ") {
                        if let Some(n) = part.strip_prefix("ahead ") {
                            ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix("behind ") {
                            behind = n.parse().unwrap_or(0);
                        }
                    }
                }
            } else {
                branch = Some(rest.to_string());
            }
        } else if line.len() >= 3 {
            let status = &line[0..2];
            let path = line[3..].trim().to_string();
            if status.starts_with('R') {
                if let Some((old, new)) = path.split_once(" -> ") {
                    entries.push(GitStatusEntry {
                        path: new.to_string(),
                        status: status.trim().to_string(),
                        old_path: Some(old.to_string()),
                    });
                    continue;
                }
            }
            entries.push(GitStatusEntry {
                path,
                status: status.trim().to_string(),
                old_path: None,
            });
        }
    }
    Ok(GitStatus {
        branch,
        upstream,
        ahead,
        behind,
        entries,
        raw,
        is_repo: true,
    })
}

/// `git diff --no-color` → unified diff text (or staged if `staged=true`).
#[tauri::command]
pub async fn reflect_git_diff(staged: Option<bool>) -> CommandResult<String> {
    let args: Vec<&str> = if staged.unwrap_or(false) {
        vec!["diff", "--no-color", "--staged"]
    } else {
        vec!["diff", "--no-color"]
    };
    let out = run_git(&args).map_err(CommandError::from)?;
    if !out.status.success() {
        return Err(CommandError {
            msg: format!("git diff failed: {}", String::from_utf8_lossy(&out.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// `git log --pretty=format:... -n N` → structured entries.
#[tauri::command]
pub async fn reflect_git_log(limit: Option<usize>) -> CommandResult<Vec<GitLogEntry>> {
    let n = limit.unwrap_or(20).to_string();
    let fmt = "%H%x1f%h%x1f%an%x1f%at%x1f%s";
    let out = run_git(&["log", &format!("--pretty=format:{fmt}"), "-n", &n])
        .map_err(CommandError::from)?;
    if !out.status.success() {
        return Err(CommandError {
            msg: format!("git log failed: {}", String::from_utf8_lossy(&out.stderr)),
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\u{1f}').collect();
        if parts.len() < 5 {
            continue;
        }
        let ts: i64 = parts[3].parse().unwrap_or(0);
        entries.push(GitLogEntry {
            hash: parts[0].to_string(),
            short_hash: parts[1].to_string(),
            author: parts[2].to_string(),
            timestamp: ts,
            subject: parts[4].to_string(),
        });
    }
    Ok(entries)
}
