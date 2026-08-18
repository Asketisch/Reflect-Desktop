//! 知识管理系统(KMS)命令。
//!
//! 基于 grep 的 wiki + /dream 会话挖掘。
//! 包装 `reflect_app_core::kms::KnowledgeManager`。

use reflect_app_core::kms::{DreamResult, KmsError, Page, SearchResult, WikiInfo};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

impl From<KmsError> for CommandError {
    fn from(e: KmsError) -> Self {
        CommandError { msg: e.to_string() }
    }
}

/// 列出所有知识库。
#[tauri::command]
pub async fn reflect_kms_list(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<WikiInfo>> {
    Ok(agent.kms_manager().list_wikis())
}

/// 创建一个新的知识库。
#[tauri::command]
pub async fn reflect_kms_create(
    agent: State<'_, MinimalAgent>,
    name: String,
    description: Option<String>,
) -> CommandResult<WikiInfo> {
    agent
        .kms_manager()
        .create_wiki(&name, description)
        .map_err(CommandError::from)
}

/// 删除一个知识库。
#[tauri::command]
pub async fn reflect_kms_delete(
    agent: State<'_, MinimalAgent>,
    name: String,
) -> CommandResult<()> {
    agent
        .kms_manager()
        .delete_wiki(&name)
        .map_err(CommandError::from)
}

/// 在知识库中保存一个页面。
#[tauri::command]
pub async fn reflect_kms_save_page(
    agent: State<'_, MinimalAgent>,
    wiki: String,
    page: String,
    content: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
) -> CommandResult<Page> {
    agent
        .kms_manager()
        .save_page(&wiki, &page, &content, title, tags.unwrap_or_default())
        .map_err(CommandError::from)
}

/// 从知识库读取一个页面。
#[tauri::command]
pub async fn reflect_kms_get_page(
    agent: State<'_, MinimalAgent>,
    wiki: String,
    page: String,
) -> CommandResult<Page> {
    agent
        .kms_manager()
        .get_page(&wiki, &page)
        .map_err(CommandError::from)
}

/// 列出知识库中的所有页面。
#[tauri::command]
pub async fn reflect_kms_list_pages(
    agent: State<'_, MinimalAgent>,
    wiki: String,
) -> CommandResult<Vec<Page>> {
    agent
        .kms_manager()
        .list_pages(&wiki)
        .map_err(CommandError::from)
}

/// 在所有知识库内搜索。
#[tauri::command]
pub async fn reflect_kms_search(
    agent: State<'_, MinimalAgent>,
    query: String,
) -> CommandResult<Vec<SearchResult>> {
    Ok(agent.kms_manager().search(&query))
}

/// 运行一次 /dream 会话 —— 从最近的会话中抽取洞察。
#[tauri::command]
pub async fn reflect_dream(
    agent: State<'_, MinimalAgent>,
    insights: Vec<String>,
    sessions_analyzed: usize,
) -> CommandResult<DreamResult> {
    agent
        .kms_manager()
        .dream(insights, sessions_analyzed)
        .map_err(CommandError::from)
}