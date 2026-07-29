//! Autopilot configuration and state.
//!
//! The Autopilot allows automatic task scheduling based on cron-like triggers.
//! It integrates with the existing Schedule system to automatically create and
//! execute tasks based on configurable templates.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the Autopilot system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotConfig {
    /// Whether the Autopilot is enabled.
    pub enabled: bool,
    /// Cron schedule for automatic task creation.
    pub schedule: String,
    /// Template prompt for automatically created tasks.
    pub task_template: String,
    /// Agent to use for executing tasks.
    pub agent: Option<String>,
    /// Maximum number of concurrent autopilot tasks.
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

/// A history entry for Autopilot runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotRun {
    /// Unique run identifier.
    pub id: String,
    /// Timestamp when the run was executed.
    pub executed_at_ms: u64,
    /// Task ID that was created.
    pub task_id: Option<String>,
    /// Status of the run.
    pub status: AutopilotRunStatus,
    /// Optional error message.
    pub error: Option<String>,
}

/// Status of an Autopilot run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutopilotRunStatus {
    /// The run completed successfully.
    Success,
    /// The run failed with an error.
    Failed,
    /// The run is still in progress.
    InProgress,
    /// The run was skipped (max concurrent reached).
    Skipped,
}

/// Autopilot manager that handles configuration and history.
pub struct AutopilotManager {
    /// Path to the configuration file.
    config_path: PathBuf,
    /// Path to the history file.
    history_path: PathBuf,
}

impl AutopilotManager {
    /// Create a new AutopilotManager with the default paths.
    pub fn new() -> Self {
        let root = dirs::home_dir()
            .map(|h| h.join(".reflect").join("autopilot"))
            .unwrap_or_else(|| PathBuf::from(".reflect/autopilot"));
        Self::new_with_root(root)
    }

    /// Create a new AutopilotManager with an explicit root directory (test-only).
    ///
    /// `root` 不需要预先存在;`save_*` 会 `create_dir_all`。
    pub fn new_with_root(root: PathBuf) -> Self {
        std::fs::create_dir_all(&root).ok();
        Self {
            config_path: root.join("config.json"),
            history_path: root.join("history.json"),
        }
    }

    /// Load or create default configuration.
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

    /// Save configuration to disk.
    pub fn save_config(&self, config: &AutopilotConfig) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Load run history.
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

    /// Save run history.
    pub fn save_history(&self, history: &[AutopilotRun]) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(history)?;
        std::fs::write(&self.history_path, content)?;
        Ok(())
    }

    /// Add a new run to history.
    pub fn add_run(&self, run: AutopilotRun) -> Result<(), std::io::Error> {
        let mut history = self.load_history();
        history.insert(0, run);
        // Keep only last 100 runs
        if history.len() > 100 {
            history.truncate(100);
        }
        self.save_history(&history)
    }

    /// Check if a run should execute based on current config.
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
        // leak the tempdir so the files survive the test (test 进程退出时清理)。
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