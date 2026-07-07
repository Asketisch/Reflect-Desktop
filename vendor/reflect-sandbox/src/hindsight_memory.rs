//! Hindsight Memory stub —— 事后反思记忆层占位。

use serde::{Deserialize, Serialize};

/// 一条 hindsight 反思记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HindsightEntry {
    pub session_id: String,
    pub lesson: String,
    pub tags: Vec<String>,
}

/// 状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HindsightStubStatus {
    Empty,
    HasEntries,
}

/// Hindsight 内存 stub。
#[derive(Debug, Default, Clone)]
pub struct HindsightMemory {
    entries: Vec<HindsightEntry>,
}

impl HindsightMemory {
    pub fn record(&mut self, entry: HindsightEntry) {
        self.entries.push(entry);
    }

    pub fn list(&self) -> &[HindsightEntry] {
        &self.entries
    }

    pub fn status(&self) -> HindsightStubStatus {
        if self.entries.is_empty() {
            HindsightStubStatus::Empty
        } else {
            HindsightStubStatus::HasEntries
        }
    }

    pub fn status_line(&self) -> String {
        match self.status() {
            HindsightStubStatus::Empty => {
                "hindsight-memory: stub — 无反思条目(v2.x 向量检索真实化)".into()
            }
            HindsightStubStatus::HasEntries => format!(
                "hindsight-memory: stub — {} 条 lesson 已记录",
                self.entries.len()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments() {
        let mut h = HindsightMemory::default();
        h.record(HindsightEntry {
            session_id: "s1".into(),
            lesson: "always test".into(),
            tags: vec!["testing".into()],
        });
        assert_eq!(h.status(), HindsightStubStatus::HasEntries);
    }
}
