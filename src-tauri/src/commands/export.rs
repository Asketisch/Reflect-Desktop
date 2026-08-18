//! Session 导出为 Markdown (`reflect_export_session_markdown`)。
//!
//! 把 session replay flatten 成 markdown transcript,写到 `~/.reflect/exports/<id>.md`。

use serde::{Deserialize, Serialize};

use reflect_protocol::ThreadId;
use reflect_rollout::reader as rollout_reader;

use crate::commands::error::{CommandError, CommandResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkdownExportResult {
    pub path: String,
    pub bytes: u64,
}

/// 回放 session,flatten 成 Markdown 转写稿,写入
/// `~/.reflect/exports/<id>.md`,返回绝对路径。
#[tauri::command]
pub async fn reflect_export_session_markdown(id: ThreadId) -> CommandResult<MarkdownExportResult> {
    use reflect_protocol::RolloutRecord;

    let home = dirs::home_dir().ok_or_else(|| CommandError {
        msg: "no HOME dir".into(),
    })?;
    let base = home.join(".reflect/sessions");
    let records = rollout_reader::replay(&base, id)
        .await
        .map_err(CommandError::from)?;

    let mut out = String::new();
    out.push_str(&format!("# Reflect session `{id}`\n\n"));
    let mut session_meta_emitted = false;
    let mut count: usize = 0;
    for r in &records {
        match r {
            RolloutRecord::SessionMeta {
                session_id: _,
                model,
                started_at: _,
            } => {
                if !session_meta_emitted {
                    out.push_str(&format!("**model**: `{model}`\n\n"));
                    session_meta_emitted = true;
                }
            }
            RolloutRecord::Message {
                turn_id: _,
                role,
                content,
            } => {
                count += 1;
                // `content` 是 serde_json::Value;字符串直接当作文本。
                let text = match content {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let label = match role {
                    reflect_protocol::MessageRole::User => "**user**",
                    reflect_protocol::MessageRole::Assistant => "**agent**",
                    reflect_protocol::MessageRole::System => "**system**",
                    reflect_protocol::MessageRole::Tool => "**tool**",
                };
                out.push_str(&format!("\n## Turn {count}\n\n{label}: "));
                out.push_str(&text);
                out.push_str("\n\n");
            }
            RolloutRecord::Compaction {
                strategy,
                removed_count,
                summary,
                ..
            } => {
                out.push_str(&format!(
                    "\n---\n*compaction* (`{strategy}`, removed {removed_count} messages)\n\n"
                ));
                if !summary.is_empty() {
                    out.push_str("> ");
                    out.push_str(summary);
                    out.push_str("\n\n");
                }
            }
            RolloutRecord::Fork {
                parent_session_id,
                branch_name,
            } => {
                out.push_str(&format!(
                    "\n*forked from `{parent_session_id}` as `{branch_name}`*\n\n"
                ));
            }
            RolloutRecord::Checkpoint { sha, label, .. } => {
                let tag = label.clone().unwrap_or_else(|| "checkpoint".to_string());
                out.push_str(&format!("\n*checkpoint `{tag}` @ {sha}*\n\n"));
            }
            RolloutRecord::Rewind {
                target_sha,
                from_sha,
                at: _,
                ..
            } => {
                out.push_str(&format!("\n*rewind* (`{from_sha}` → `{target_sha}`)\n\n"));
            }
            RolloutRecord::DiscussionTranscript {
                mode,
                participants,
                transcript,
                ..
            } => {
                out.push_str(&format!(
                    "\n## Discussion ({mode})\n\nParticipants: {}\n\n",
                    participants.join(", ")
                ));
                let pretty = match transcript {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string_pretty(other).unwrap_or_default(),
                };
                out.push_str(&pretty);
                out.push_str("\n\n");
            }
            // v1.x:per-turn token 用量快照(累计自 EventMsg::TokenCount)。
            // Markdown 导出不渲染 token 统计 —— 显式忽略以保证 match 穷尽
            // (新增 RolloutRecord variant 不会再次打破编译)。
            RolloutRecord::TokenCount { .. } => {}
            // Plan 工作流与权限模式变更记录:Markdown 转写稿不渲染,
            // 显式忽略以保证 match 穷尽。
            RolloutRecord::PlanRequest { .. }
            | RolloutRecord::PlanReady { .. }
            | RolloutRecord::PlanRejected { .. }
            | RolloutRecord::PermissionModeChanged { .. } => {}
        }
    }

    let export_dir = home.join(".reflect/exports");
    std::fs::create_dir_all(&export_dir).map_err(CommandError::from)?;
    let dest = export_dir.join(format!("{id}.md"));
    std::fs::write(&dest, &out).map_err(CommandError::from)?;
    Ok(MarkdownExportResult {
        path: dest.to_string_lossy().into_owned(),
        bytes: out.len() as u64,
    })
}
