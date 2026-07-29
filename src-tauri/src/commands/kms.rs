//! Knowledge Management System (KMS) commands.
//!
//! Phase 3 item 12: grep-based wiki + /dream session mining.
//! Wraps `reflect_app_core::kms::KnowledgeManager`.

use reflect_app_core::kms::{DreamResult, KmsError, Page, SearchResult, WikiInfo};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

impl From<KmsError> for CommandError {
    fn from(e: KmsError) -> Self {
        CommandError { msg: e.to_string() }
    }
}

/// List all knowledge bases.
#[tauri::command]
pub async fn reflect_kms_list(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<WikiInfo>> {
    Ok(agent.kms_manager().list_wikis())
}

/// Create a new knowledge base.
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

/// Delete a knowledge base.
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

/// Save a page to a knowledge base.
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

/// Get a page from a knowledge base.
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

/// List all pages in a knowledge base.
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

/// Search across all knowledge bases.
#[tauri::command]
pub async fn reflect_kms_search(
    agent: State<'_, MinimalAgent>,
    query: String,
) -> CommandResult<Vec<SearchResult>> {
    Ok(agent.kms_manager().search(&query))
}

/// Run a /dream session - extract insights from recent sessions.
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