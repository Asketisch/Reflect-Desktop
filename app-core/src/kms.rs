//! 知识管理系统 (KMS) —— 基于 grep 的本地知识库 wiki。
//!
//! 知识库以 Markdown 文件存储于 `~/.reflect/kms/<name>/`
//! 各页面位于 `pages/*.md`。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 知识库的元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInfo {
    /// Wiki 名称（目录名）。
    pub name: String,
    /// 可选描述。
    pub description: Option<String>,
    /// Wiki 目录的路径。
    pub path: PathBuf,
    /// Wiki 中的页面数。
    pub page_count: usize,
    /// 最后修改时间戳（自 epoch 起的毫秒数）。
    pub modified_at_ms: u64,
}

/// 知识库中的单个页面。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// 页面名称（不含 .md 的文件名）。
    pub name: String,
    /// 此页面所属的 Wiki。
    pub wiki: String,
    /// 页面内容（Markdown）。
    pub content: String,
    /// 来自 frontmatter 的可选标题。
    pub title: Option<String>,
    /// 来自 frontmatter 的可选标签。
    pub tags: Vec<String>,
    /// 最后修改时间戳（自 epoch 起的毫秒数）。
    pub modified_at_ms: u64,
}

/// 知识库 grep 的搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Wiki 名称。
    pub wiki: String,
    /// 页面名称。
    pub page: String,
    /// 匹配的行。
    pub lines: Vec<String>,
    /// 行数。
    pub line_count: usize,
}

/// /dream 命令的会话挖掘结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DreamResult {
    /// 提取的洞察。
    pub insights: Vec<String>,
    /// 已分析的会话数。
    pub sessions_analyzed: usize,
    /// 洞察保存到的 Wiki 名称。
    pub wiki: String,
    /// 洞察保存到的页面名称。
    pub page: String,
}

/// KMS 操作的错误类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KmsError {
    /// 未找到 Wiki。
    WikiNotFound {
        /// Wiki 名称。
        name: String,
    },
    /// 未找到页面。
    PageNotFound {
        /// Wiki 名称。
        wiki: String,
        /// 页面名称。
        page: String,
    },
    /// Wiki 已存在。
    WikiAlreadyExists {
        /// Wiki 名称。
        name: String,
    },
    /// I/O 错误。
    IoError {
        /// 错误信息。
        message: String,
    },
}

impl std::fmt::Display for KmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KmsError::WikiNotFound { name } => write!(f, "Wiki not found: {}", name),
            KmsError::PageNotFound { wiki, page } => write!(f, "Page not found: {}/{}", wiki, page),
            KmsError::WikiAlreadyExists { name } => write!(f, "Wiki already exists: {}", name),
            KmsError::IoError { message } => write!(f, "I/O error: {}", message),
        }
    }
}

impl std::error::Error for KmsError {}

impl From<std::io::Error> for KmsError {
    fn from(e: std::io::Error) -> Self {
        KmsError::IoError { message: e.to_string() }
    }
}

/// 处理所有知识库操作的 KMS 管理器。
pub struct KnowledgeManager {
    /// 所有知识库的根目录。
    root: PathBuf,
}

impl KnowledgeManager {
    /// 使用指定的根目录创建新的 KnowledgeManager。
    pub fn new(root: PathBuf) -> Self {
        std::fs::create_dir_all(&root).ok();
        Self { root }
    }

    /// 使用默认主目录创建新的 KnowledgeManager。
    pub fn with_default_home() -> Self {
        let root = dirs::home_dir()
            .map(|h| h.join(".reflect").join("kms"))
            .unwrap_or_else(|| PathBuf::from(".reflect/kms"));
        Self::new(root)
    }

    /// 获取 Wiki 目录的路径。
    pub fn wiki_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// 获取 Wiki 内页面文件的路径。
    pub fn page_path(&self, wiki: &str, page: &str) -> PathBuf {
        self.root.join(wiki).join("pages").join(format!("{}.md", page))
    }

    /// 创建新的知识库。
    pub fn create_wiki(&self, name: &str, description: Option<String>) -> Result<WikiInfo, KmsError> {
        let wiki_path = self.wiki_path(name);
        if wiki_path.exists() {
            return Err(KmsError::WikiAlreadyExists { name: name.to_string() });
        }

        std::fs::create_dir_all(&wiki_path)?;
        std::fs::create_dir_all(wiki_path.join("pages"))?;

        let mut index_content = String::from("---\n");
        if let Some(desc) = &description {
            index_content.push_str(&format!("description: {}\n", desc));
        }
        index_content.push_str("---\n");
        std::fs::write(wiki_path.join("index.md"), index_content)?;

        let modified_at_ms = get_modified_at_ms(&wiki_path);

        Ok(WikiInfo {
            name: name.to_string(),
            description,
            path: wiki_path,
            page_count: 0,
            modified_at_ms,
        })
    }

    /// 删除知识库。
    pub fn delete_wiki(&self, name: &str) -> Result<(), KmsError> {
        let wiki_path = self.wiki_path(name);
        if !wiki_path.exists() {
            return Err(KmsError::WikiNotFound { name: name.to_string() });
        }
        std::fs::remove_dir_all(&wiki_path)?;
        Ok(())
    }

    /// 列出所有知识库。
    pub fn list_wikis(&self) -> Vec<WikiInfo> {
        let mut wikis = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return wikis;
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let name = name.to_string();

            let pages_dir = path.join("pages");
            let page_count = count_md_files(&pages_dir);

            let description = if path.join("index.md").exists() {
                std::fs::read_to_string(path.join("index.md")).ok().as_deref().and_then(extract_description)
            } else {
                None
            };

            let modified_at_ms = get_modified_at_ms(&path);

            wikis.push(WikiInfo {
                name: name.clone(),
                description,
                path: path.clone(),
                page_count,
                modified_at_ms,
            });
        }
        wikis
    }

    /// 将页面保存到知识库。
    pub fn save_page(
        &self,
        wiki: &str,
        page: &str,
        content: &str,
        title: Option<String>,
        tags: Vec<String>,
    ) -> Result<Page, KmsError> {
        let wiki_path = self.wiki_path(wiki);
        if !wiki_path.exists() {
            return Err(KmsError::WikiNotFound { name: wiki.to_string() });
        }

        std::fs::create_dir_all(wiki_path.join("pages"))?;

        let mut full_content = String::from("---\n");
        if let Some(t) = &title {
            full_content.push_str(&format!("title: {}\n", t));
        }
        if !tags.is_empty() {
            let tags_str = tags.join(", ");
            full_content.push_str(&format!("tags: [{}]\n", tags_str));
        }
        full_content.push_str("---\n");
        full_content.push_str(content);

        let page_path = self.page_path(wiki, page);
        std::fs::write(&page_path, &full_content)?;

        let modified_at_ms = get_modified_at_ms(&page_path);

        Ok(Page {
            name: page.to_string(),
            wiki: wiki.to_string(),
            content: full_content,
            title,
            tags,
            modified_at_ms,
        })
    }

    /// 从知识库获取页面。
    pub fn get_page(&self, wiki: &str, page: &str) -> Result<Page, KmsError> {
        let page_path = self.page_path(wiki, page);
        if !page_path.exists() {
            return Err(KmsError::PageNotFound {
                wiki: wiki.to_string(),
                page: page.to_string(),
            });
        }

        let content = std::fs::read_to_string(&page_path)?;
        let title = extract_title(&content);
        let tags = extract_tags(&content);
        let modified_at_ms = get_modified_at_ms(&page_path);

        Ok(Page {
            name: page.to_string(),
            wiki: wiki.to_string(),
            content,
            title,
            tags,
            modified_at_ms,
        })
    }

    /// 列出知识库中的所有页面。
    pub fn list_pages(&self, wiki: &str) -> Result<Vec<Page>, KmsError> {
        let wiki_path = self.wiki_path(wiki);
        if !wiki_path.exists() {
            return Err(KmsError::WikiNotFound { name: wiki.to_string() });
        }

        let pages_dir = wiki_path.join("pages");
        let mut pages = Vec::new();

        let Ok(entries) = std::fs::read_dir(&pages_dir) else {
            return Ok(pages);
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let title = extract_title(&content);
            let tags = extract_tags(&content);
            let modified_at_ms = get_modified_at_ms(&path);

            pages.push(Page {
                name: name.to_string(),
                wiki: wiki.to_string(),
                content,
                title,
                tags,
                modified_at_ms,
            });
        }

        Ok(pages)
    }

    /// 在所有知识库中搜索内容。
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return results;
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let wiki_path = entry.path();
            if !wiki_path.is_dir() {
                continue;
            }
            let Some(wiki_name) = wiki_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let wiki_name = wiki_name.to_string();

            let pages_dir = wiki_path.join("pages");
            let Ok(page_entries) = std::fs::read_dir(&pages_dir) else {
                continue;
            };

            for page_entry in page_entries.filter_map(|e| e.ok()) {
                let page_path = page_entry.path();
                if !page_path.is_file() || page_path.extension().is_none_or(|e| e != "md") {
                    continue;
                }
                let Some(page_name) = page_path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let page_name = page_name.to_string();
                let Ok(content) = std::fs::read_to_string(&page_path) else {
                    continue;
                };

                let mut matching_lines: Vec<String> = Vec::new();
                for line in content.lines() {
                    if line.contains(query) {
                        matching_lines.push(line.to_string());
                    }
                }

                if !matching_lines.is_empty() {
                    let lc = matching_lines.len();
                    results.push(SearchResult {
                        wiki: wiki_name.clone(),
                        page: page_name,
                        lines: matching_lines,
                        line_count: lc,
                    });
                }
            }
        }

        results
    }

    /// 运行 dream 会话 - 从最近的会话中提取洞察。
    pub fn dream(
        &self,
        insights: Vec<String>,
        sessions_analyzed: usize,
    ) -> Result<DreamResult, KmsError> {
        let insights_wiki = "dream-insights";
        if !self.wiki_path(insights_wiki).exists() {
            self.create_wiki(
                insights_wiki,
                Some("Auto-generated insights from sessions".to_string()),
            )?;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let page_name = format!("insights-{}", timestamp);

        let mut content = String::from("# Session Insights\n\n");
        for (i, insight) in insights.iter().enumerate() {
            content.push_str(&format!("{}. {}\n\n", i + 1, insight));
        }

        self.save_page(
            insights_wiki,
            &page_name,
            &content,
            Some(format!("Insights {}", timestamp)),
            vec!["dream".to_string()],
        )?;

        Ok(DreamResult {
            insights,
            sessions_analyzed,
            wiki: insights_wiki.to_string(),
            page: page_name,
        })
    }
}

/// 获取路径自 epoch 起的修改时间戳（毫秒）。
fn get_modified_at_ms(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 统计目录中的 .md 文件数。
fn count_md_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_file() && p.extension().map_or(false, |ext| ext == "md")
                })
                .count()
        })
        .unwrap_or(0)
}

/// 从 Markdown frontmatter 中提取描述。
fn extract_description(content: &str) -> Option<String> {
    let mut lines = content.lines();
    let first_line = lines.next().unwrap_or_default();
    if first_line != "---" {
        return None;
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if line.starts_with("description: ") {
            let desc = line.trim_start_matches("description: ").trim().to_string();
            if !desc.is_empty() {
                return Some(desc);
            }
        }
    }
    None
}

/// 从 Markdown frontmatter 中提取标题。
fn extract_title(content: &str) -> Option<String> {
    let mut lines = content.lines();
    let first_line = lines.next().unwrap_or_default();
    if first_line != "---" {
        return None;
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if line.starts_with("title: ") {
            let title = line.trim_start_matches("title: ").trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }
    None
}

/// 从 Markdown frontmatter 中提取标签。
fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut lines = content.lines();
    let first_line = lines.next().unwrap_or_default();
    if first_line != "---" {
        return tags;
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if line.starts_with("tags: ") {
            let tags_str = line.trim_start_matches("tags: ").trim();
            let tags_str = tags_str.trim_matches('[').trim_matches(']');
            tags = tags_str
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> (KnowledgeManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let manager = KnowledgeManager::new(dir.path().to_path_buf());
        (manager, dir)
    }

    #[test]
    fn create_and_list_wiki() {
        let (manager, _dir) = create_test_manager();
        let wiki = manager.create_wiki("test-wiki", Some("Test wiki".to_string())).unwrap();
        assert_eq!(wiki.name, "test-wiki");
        assert_eq!(wiki.page_count, 0);

        let wikis = manager.list_wikis();
        assert_eq!(wikis.len(), 1);
        assert_eq!(wikis[0].name, "test-wiki");
    }

    #[test]
    fn create_wiki_already_exists_error() {
        let (manager, _dir) = create_test_manager();
        manager.create_wiki("test-wiki", None).unwrap();
        let result = manager.create_wiki("test-wiki", None);
        assert!(matches!(result, Err(KmsError::WikiAlreadyExists { .. })));
    }

    #[test]
    fn save_and_get_page() {
        let (manager, _dir) = create_test_manager();
        manager.create_wiki("test-wiki", None).unwrap();

        let page = manager
            .save_page(
                "test-wiki",
                "hello",
                "# Hello World\nThis is a test page.",
                Some("Hello".to_string()),
                vec!["test".to_string()],
            )
            .unwrap();

        assert_eq!(page.name, "hello");
        assert_eq!(page.wiki, "test-wiki");
        assert_eq!(page.title.as_deref(), Some("Hello"));
        assert_eq!(page.tags, vec!["test"]);
        assert!(page.content.contains("# Hello World"));
    }

    #[test]
    fn list_pages() {
        let (manager, _dir) = create_test_manager();
        manager.create_wiki("test-wiki", None).unwrap();
        manager.save_page("test-wiki", "page1", "Content 1", None, vec![]).unwrap();
        manager.save_page("test-wiki", "page2", "Content 2", None, vec![]).unwrap();

        let pages = manager.list_pages("test-wiki").unwrap();
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn search_pages() {
        let (manager, _dir) = create_test_manager();
        manager.create_wiki("test-wiki", None).unwrap();
        manager.save_page("test-wiki", "hello", "# Hello World\nThis is a test page.", None, vec![]).unwrap();
        manager.save_page("test-wiki", "goodbye", "# Goodbye\nThis is another page.", None, vec![]).unwrap();

        let results = manager.search("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].wiki, "test-wiki");
        assert_eq!(results[0].page, "hello");
    }

    #[test]
    fn delete_wiki() {
        let (manager, _dir) = create_test_manager();
        manager.create_wiki("test-wiki", None).unwrap();
        manager.delete_wiki("test-wiki").unwrap();

        let wikis = manager.list_wikis();
        assert_eq!(wikis.len(), 0);
    }
}