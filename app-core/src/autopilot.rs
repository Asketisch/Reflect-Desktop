//! Autopilot 配置与状态。
//!
//! Autopilot 支持基于类 cron 触发器的任务自动调度。
//! 它与现有 Schedule 系统集成，根据可配置模板自动创建并
//! 执行任务。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Autopilot 系统配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotConfig {
    /// 是否启用 Autopilot。
    pub enabled: bool,
    /// 自动创建任务的 cron 调度。
    pub schedule: String,
    /// 自动创建任务使用的模板 prompt。
    pub task_template: String,
    /// 执行任务所用的 agent。
    pub agent: Option<String>,
    /// Autopilot 任务的最大并发数。
    pub max_concurrent: usize,
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            schedule: String::new(),
            task_template: String::new(),
            agent: None,
            max_concurrent: 1,
        }
    }
}

/// Autopilot 运行历史记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotRun {
    /// 唯一的运行标识符。
    pub id: String,
    /// 运行执行时的时间戳。
    pub executed_at_ms: u64,
    /// 创建的任务 ID。
    pub task_id: Option<String>,
    /// 运行状态。
    pub status: AutopilotRunStatus,
    /// 可选的错误消息。
    pub error: Option<String>,
}

/// Autopilot 运行状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutopilotRunStatus {
    /// 运行成功完成。
    Success,
    /// 运行因错误失败。
    Failed,
    /// 运行仍在进行中。
    InProgress,
    /// 运行已跳过（已达到最大并发数）。
    Skipped,
}

/// 处理配置与历史记录的 Autopilot 管理器。
pub struct AutopilotManager {
    /// 配置文件路径。
    config_path: PathBuf,
    /// 历史记录文件路径。
    history_path: PathBuf,
}

impl AutopilotManager {
    /// 使用默认路径创建 AutopilotManager。
    pub fn new() -> Self {
        let root = dirs::home_dir()
            .map(|h| h.join(".reflect").join("autopilot"))
            .unwrap_or_else(|| PathBuf::from(".reflect/autopilot"));
        Self::new_with_root(root)
    }

    /// 使用指定根目录创建 AutopilotManager（仅用于测试）。
    ///
    /// `root` 不需要预先存在;`save_*` 会 `create_dir_all`。
    pub fn new_with_root(root: PathBuf) -> Self {
        std::fs::create_dir_all(&root).ok();
        Self {
            config_path: root.join("config.json"),
            history_path: root.join("history.json"),
        }
    }

    /// 加载配置；不存在或无效时创建默认配置。
    pub fn load_config(&self) -> AutopilotConfig {
        if self.config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        AutopilotConfig::default()
    }

    /// 将配置保存到磁盘。
    pub fn save_config(&self, config: &AutopilotConfig) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// 加载运行历史。
    pub fn load_history(&self) -> Vec<AutopilotRun> {
        if self.history_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.history_path) {
                if let Ok(history) = serde_json::from_str(&content) {
                    return history;
                }
            }
        }
        Vec::new()
    }

    /// 保存运行历史。
    pub fn save_history(&self, history: &[AutopilotRun]) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(history)?;
        std::fs::write(&self.history_path, content)?;
        Ok(())
    }

    /// 向历史记录添加一次新运行。
    pub fn add_run(&self, run: AutopilotRun) -> Result<(), std::io::Error> {
        let mut history = self.load_history();
        history.insert(0, run);
        // 仅保留最近 100 次运行。
        if history.len() > 100 {
            history.truncate(100);
        }
        self.save_history(&history)
    }

    /// 根据当前配置检查是否应执行运行。
    pub fn should_run(&self) -> bool {
        let config = self.load_config();
        config.enabled && !config.schedule.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试用独立 tempdir,避免污染真实 `~/.reflect/autopilot/`(生产态)。
    fn fresh_manager() -> AutopilotManager {
        let dir = tempfile::tempdir().expect("tempdir");
        // 泄漏 tempdir 使文件在测试期间存活（test 进程退出时清理）。
        let path = dir.into_path();
        AutopilotManager::new_with_root(path)
    }

    #[test]
    fn default_config_is_disabled() {
        let config = AutopilotConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_concurrent, 1);
    }

    #[test]
    fn load_config_creates_default() {
        let manager = fresh_manager();
        let config = manager.load_config();
        assert!(!config.enabled);
    }

    #[test]
    fn save_and_load_config() {
        let manager = fresh_manager();
        let mut config = AutopilotConfig::default();
        config.enabled = true;
        config.schedule = "0 * * * *".to_string();
        manager.save_config(&config).unwrap();

        let loaded = manager.load_config();
        assert!(loaded.enabled);
        assert_eq!(loaded.schedule, "0 * * * *");
    }

    #[test]
    fn history_starts_empty() {
        let manager = fresh_manager();
        let history = manager.load_history();
        assert!(history.is_empty());
    }
}