//! 持久化 eval cells stub —— Jupyter-style 可复用 eval 块。

use serde::{Deserialize, Serialize};

/// 一条 eval cell 记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalCell {
    pub id: String,
    pub language: String,
    pub source: String,
    #[serde(default)]
    pub last_output: Option<String>,
}

/// Store 状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalCellStubStatus {
    Empty,
    HasCells,
}

/// Eval cell 内存 stub store。
#[derive(Debug, Default, Clone)]
pub struct EvalCellStore {
    cells: Vec<EvalCell>,
}

impl EvalCellStore {
    pub fn upsert(&mut self, cell: EvalCell) {
        if let Some(i) = self.cells.iter().position(|c| c.id == cell.id) {
            self.cells[i] = cell;
        } else {
            self.cells.push(cell);
        }
    }

    pub fn get(&self, id: &str) -> Option<&EvalCell> {
        self.cells.iter().find(|c| c.id == id)
    }

    pub fn list(&self) -> &[EvalCell] {
        &self.cells
    }

    pub fn status(&self) -> EvalCellStubStatus {
        if self.cells.is_empty() {
            EvalCellStubStatus::Empty
        } else {
            EvalCellStubStatus::HasCells
        }
    }
}

/// 占位执行(不真正跑代码)。
pub fn eval_cell_stub(cell_id: &str) -> String {
    format!("eval-cells stub: run({cell_id}) — 持久化 eval 执行留 v2.x")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_and_get() {
        let mut s = EvalCellStore::default();
        s.upsert(EvalCell {
            id: "c1".into(),
            language: "python".into(),
            source: "1+1".into(),
            last_output: None,
        });
        assert_eq!(s.get("c1").unwrap().source, "1+1");
    }
}
