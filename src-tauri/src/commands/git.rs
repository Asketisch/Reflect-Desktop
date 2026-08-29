//! Git 集成 —— status / diff / log。

use std::path::PathBuf;
use std::process::Command as StdCommand;

use serde::{Deserialize, Serialize};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: String, // "M" | "A" | "D" | "??" | "R"
    /// 该条目是否含已暂存(index)变更。`status` 是 trimmed 的 porcelain
    /// 码,`"M "`(已暂存)与 `" M"`(仅工作区)折叠成同一个 "M" —— 前端
    /// staged/working 分 tab 依赖本字段区分。
    pub staged: bool,
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

/// `git status --porcelain=v1 --branch` → 结构化 status。
#[tauri::command]
pub async fn reflect_git_status() -> CommandResult<GitStatus> {
    // 首先确认这是否真的是 git 仓库。
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
            // 分支头行:"## <branch>" 或 "## <branch>...<upstream> [ahead N, behind M]"
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
                        staged: !is_worktree_only(status),
                        old_path: Some(old.to_string()),
                    });
                    continue;
                }
            }
            entries.push(GitStatusEntry {
                path,
                status: status.trim().to_string(),
                staged: !is_worktree_only(status),
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

/// porcelain v1 两字符码的**第一字符**(index 列)是否为「无暂存变更」:
/// `' '`(仅工作区修改)或 `'?'`(untracked)。第二字符是工作区列,
/// 不参与本判定。
fn is_worktree_only(status: &str) -> bool {
    matches!(status.as_bytes().first(), Some(b' ') | Some(b'?'))
}

#[cfg(test)]
mod tests {
    use super::is_worktree_only;

    /// `"M "`(已暂存)与 `" M"`(仅工作区)经 trim 折叠成同一个 "M",
    /// staged 判定必须吃原始两字符码的 index 列。
    #[test]
    fn porcelain_index_column_drives_staged() {
        assert!(is_worktree_only(" M"));
        assert!(is_worktree_only("??"));
        assert!(!is_worktree_only("M "));
        assert!(!is_worktree_only("MM"));
        assert!(!is_worktree_only("AM"));
        assert!(!is_worktree_only("A "));
        assert!(!is_worktree_only("R "));
    }
}

/// `git diff --no-color` → unified diff 文本(`staged=true` 时为 staged diff)。
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

/// `git log --pretty=format:... -n N` → 结构化 entry 列表。
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

// ── P3：Git 操作化（stage / unstage / commit）──────────────────────────
//
// 此前 Git 视图只读；这里补齐最小写路径 —— 与 run_git 同一工作区、同一
// 错误链路。不做 push/pull（远端操作涉及凭证，仍留给用户终端）。

/// stage 文件（`git add -- <paths>`）。空 paths = add -A（全量）。
#[tauri::command]
pub async fn reflect_git_stage(paths: Vec<String>) -> CommandResult<()> {
    let output = if paths.is_empty() {
        run_git(&["add", "--all"])
    } else {
        // 路径以 `--` 分隔传入，防止以 `-` 开头的路径被解析为选项。
        let mut args: Vec<&str> = vec!["add", "--"];
        args.extend(paths.iter().map(|s| s.as_str()));
        run_git(&args)
    }?;
    if !output.status.success() {
        return Err(CommandError {
            msg: format!("git add failed: {}", String::from_utf8_lossy(&output.stderr)),
        });
    }
    Ok(())
}

/// unstage 文件（`git reset HEAD -- <paths>`；路径为空 = 全量 reset）。
#[tauri::command]
pub async fn reflect_git_unstage(paths: Vec<String>) -> CommandResult<()> {
    let output = if paths.is_empty() {
        run_git(&["reset", "HEAD"])
    } else {
        let mut args: Vec<&str> = vec!["reset", "HEAD", "--"];
        args.extend(paths.iter().map(|s| s.as_str()));
        run_git(&args)
    }?;
    if !output.status.success() {
        return Err(CommandError {
            msg: format!("git reset failed: {}", String::from_utf8_lossy(&output.stderr)),
        });
    }
    Ok(())
}

/// 提交（`git commit -m <message>`）。要求非空 message；返回新 commit 短 hash。
#[tauri::command]
pub async fn reflect_git_commit(message: String) -> CommandResult<String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(CommandError { msg: "commit message is empty".into() });
    }
    let output = run_git(&["commit", "-m", trimmed])?;
    if !output.status.success() {
        return Err(CommandError {
            msg: format!("git commit failed: {}", String::from_utf8_lossy(&output.stderr)),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // 提取 `[branch abc1234] subject` 中的短 hash。
    let short = stdout
        .split('[')
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .and_then(|head| head.split_whitespace().nth(1))
        .unwrap_or("ok")
        .to_string();
    Ok(short)
}

/// 列出打开中的 GitHub PR（`gh pr list --json ...`）。gh 未安装 / 非 repo
/// / 无 PR 时返回空列表 + 说明（前端可展示提示，不打断）。
#[derive(Debug, Serialize)]
pub struct GhPullRequest {
    pub number: u64,
    pub title: String,
    pub head_ref: String,
    pub author: String,
    pub updated_at: String,
    pub url: String,
    pub draft: bool,
}

/// 列出当前工作区对应的 GitHub PR（调用 `gh pr list --json ...`）。
///
/// 失败语义:gh 未安装、非 git 仓库、未登录 gh → 返回 `Err` 带原因(由
/// PROTOCOL_BRIDGE.md §git 约定)。前端在空态中展示 `msg`。
#[tauri::command]
pub async fn reflect_gh_pr_list(limit: Option<usize>) -> CommandResult<Vec<GhPullRequest>> {
    let limit = limit.unwrap_or(20).clamp(1, 50);
    let cwd = git_in_workspace().ok_or_else(|| CommandError { msg: "no workspace".into() })?;
    let output = StdCommand::new("gh")
        .current_dir(&cwd)
        .args([
            "pr",
            "list",
            "--limit",
            &limit.to_string(),
            "--json",
            "number,title,headRefName,author,updatedAt,url,isDraft",
        ])
        .output()
        .map_err(|e| CommandError {
            msg: if e.kind() == std::io::ErrorKind::NotFound {
                "gh CLI not found — install GitHub CLI to list pull requests".into()
            } else {
                format!("spawn gh: {e}")
            },
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 非 repo / 未登录 gh / gh 未安装 → Err 带原因（前端在空态中展示）。
        return Err(CommandError { msg: stderr.trim().to_string() });
    }
    #[derive(serde::Deserialize)]
    #[allow(non_snake_case)] // 字段名直接对应 `gh pr list --json` 的 key。
    struct RawPr {
        number: u64,
        title: String,
        headRefName: String,
        author: Option<RawAuthor>,
        updatedAt: String,
        url: String,
        isDraft: bool,
    }
    #[derive(serde::Deserialize)]
    struct RawAuthor {
        login: String,
    }
    let raw: Vec<RawPr> = serde_json::from_slice(&output.stdout).map_err(|e| CommandError {
        msg: format!("parse gh output: {e}"),
    })?;
    Ok(raw
        .into_iter()
        .map(|pr| GhPullRequest {
            number: pr.number,
            title: pr.title,
            head_ref: pr.headRefName,
            author: pr.author.map(|a| a.login).unwrap_or_default(),
            updated_at: pr.updatedAt,
            url: pr.url,
            draft: pr.isDraft,
        })
        .collect())
}
