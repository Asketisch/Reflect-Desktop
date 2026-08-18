//! Activity timeline 命令面。
//!
//! 薄包装 `reflect_app_core::activity::ActivityLogger`,供前端 Inbox /
//! Activity timeline / @mention 搜索消费。

use reflect_app_core::activity::{ActivityError, ActivityEvent, ActivityFilter};
use tauri::State;

use super::CommandResult;
use crate::state::MinimalAgent;

impl From<ActivityError> for super::error::CommandError {
    fn from(e: ActivityError) -> Self {
        super::error::CommandError { msg: e.to_string() }
    }
}

/// 列出 activity 事件(按过滤条件)。
#[tauri::command]
pub async fn reflect_list_activity(
    agent: State<'_, MinimalAgent>,
    filter: Option<ActivityFilter>,
) -> CommandResult<Vec<ActivityEvent>> {
    let logger = agent.activity_logger();
    let f = filter.unwrap_or_default();
    Ok(logger.list(&f)?)
}

/// 搜索 mention(`@<query>` 出现在 summary 或 actorId 里)。
#[tauri::command]
pub async fn reflect_search_activity(
    agent: State<'_, MinimalAgent>,
    query: String,
) -> CommandResult<Vec<ActivityEvent>> {
    let logger = agent.activity_logger();
    Ok(logger.search_mentions(&query)?)
}

/// 清空内存 activity 缓冲(磁盘审计文件不动)。
#[tauri::command]
pub async fn reflect_clear_activity(agent: State<'_, MinimalAgent>) -> CommandResult<()> {
    let logger = agent.activity_logger();
    logger.clear_memory();
    Ok(())
}

/// 测试辅助:当前内存缓冲事件数(诊断用)。
#[tauri::command]
pub async fn reflect_activity_count(agent: State<'_, MinimalAgent>) -> CommandResult<usize> {
    Ok(agent.activity_logger().len())
}
