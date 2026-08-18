//! Markdown grep / 全仓库查找（`reflect_search_files`）。
//!
//! 复用 `files::should_skip` 来过滤目录。

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::commands::files::should_skip;
use crate::state::MinimalAgent;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSearchHit {
    pub path: String,
    pub line: usize,
    pub context: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub root: String,
    pub query: String,
    pub hits: Vec<FileSearchHit>,
    pub truncated: bool,
}

#[tauri::command]
pub async fn reflect_search_files(
    agent: State<'_, MinimalAgent>,
    query: String,
    path: Option<String>,
    max_results: Option<usize>,
) -> CommandResult<FileSearchResult> {
    let root = match path.as_deref() {
        Some("") | None => agent.workspace().to_path_buf(),
        Some(p) => {
            let pp = Path::new(p);
            if pp.is_absolute() {
                pp.to_path_buf()
            } else {
                agent.workspace().join(pp)
            }
        }
    };
    if !root.exists() {
        return Err(CommandError {
            msg: format!("path does not exist: {}", root.display()),
        });
    }
    let cap = max_results.unwrap_or(200);
    let mut hits = Vec::new();
    let mut truncated = false;
    search_walk(&root, &query, cap, &mut hits, &mut truncated);
    Ok(FileSearchResult {
        root: root.display().to_string(),
        query,
        hits,
        truncated,
    })
}

pub(crate) fn search_walk(
    dir: &Path,
    query: &str,
    cap: usize,
    hits: &mut Vec<FileSearchHit>,
    truncated: &mut bool,
) {
    if hits.len() >= cap {
        *truncated = true;
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if hits.len() >= cap {
            *truncated = true;
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name) {
            continue;
        }
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            search_walk(&path, query, cap, hits, truncated);
        } else if meta.is_file() && meta.len() < 512 * 1024 {
            search_file(&path, query, hits, cap, truncated);
        }
    }
}

pub(crate) fn search_file(
    path: &Path,
    query: &str,
    hits: &mut Vec<FileSearchHit>,
    cap: usize,
    truncated: &mut bool,
) {
    if hits.len() >= cap {
        *truncated = true;
        return;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for (i, line) in content.lines().enumerate() {
        if hits.len() >= cap {
            *truncated = true;
            return;
        }
        if line.to_lowercase().contains(&query.to_lowercase()) {
            hits.push(FileSearchHit {
                path: path.display().to_string(),
                line: i + 1,
                context: line.chars().take(240).collect(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_file_basic() {
        let dir = tempdir();
        let p = dir.join("sample.rs");
        std::fs::write(&p, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
        let mut hits = Vec::new();
        let mut truncated = false;
        search_file(&p, "println", &mut hits, 10, &mut truncated);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 2);
        assert!(truncated == false);
    }

    #[test]
    fn search_walk_skips_target() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/ignored.rs"), "println!();").unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() { }").unwrap();
        let mut hits = Vec::new();
        let mut truncated = false;
        search_walk(&dir, "main", 10, &mut hits, &mut truncated);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].path.ends_with("src/main.rs"));
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("reflect-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}
