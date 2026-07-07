//! `RenderState` 占位 —— M1.1 只导出空结构。
//!
//! 后续里程碑（M1.4 起）会把 `crates/reflect-tui/src/app.rs::RenderState`（line 304）
//! 及其所有字段原样迁到本模块。 TUI 端通过 `pub use reflect_app_core::state::*`
//! 重新导出，0 改动。
//!
//! 字段抽取进度见 `docs/todo/21-gui/01-mvp-scaffold.md`。

/// M1.1 占位：UI-agnostic render state。后续会把 `reflect_tui::RenderState`
/// （`crates/reflect-tui/src/app.rs:304`，约 200 字段）的字段全量迁移过来。
///
/// 设计原则：
/// - 不引用 ratatui/crossterm 类型（KeyEvent 用 `reflect_protocol` 抽象）
/// - 派生 `Default` 便于测试构造
/// - 不在此构造 IO（IO 通过 `Services` bundle 注入）
#[derive(Debug, Default, Clone)]
pub struct RenderState {
    /// 预留：当前激活 thread id（M1.3 起填充）。
    pub active_thread: Option<String>,
    /// 预留：当前 pending approval（M1.6 Modal 套件填充）。
    pub pending_approval: Option<PendingApproval>,
    /// 预留：当前 pending question（M1.6）。
    pub pending_question: Option<PendingQuestion>,
    /// 预留：当前 pending ask-user input（M1.6）。
    pub pending_ask_user: Option<PendingAskUser>,
}

/// M1.1 占位：与 TUI `PendingApproval` 对应的 UI-agnostic 版本。
#[derive(Debug, Clone)]
pub struct PendingApproval {
    /// approval id。
    pub id: String,
    /// 触发的工具名。
    pub tool_name: String,
    /// 风险等级（M1 不强制）。
    pub risk: Option<String>,
}

/// M1.1 占位：与 TUI `PendingQuestion` 对应的 UI-agnostic 版本。
#[derive(Debug, Clone)]
pub struct PendingQuestion {
    /// question prompt id。
    pub id: String,
    /// 问题文本（多题用多条）。
    pub questions: Vec<String>,
}

/// M1.1 占位：与 TUI `PendingAskUser` 对应的 UI-agnostic 版本。
#[derive(Debug, Clone)]
pub struct PendingAskUser {
    /// ask-user prompt id。
    pub id: String,
    /// 提示文本。
    pub prompt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_state_default_has_no_active_thread() {
        let s = RenderState::default();
        assert!(s.active_thread.is_none());
        assert!(s.pending_approval.is_none());
        assert!(s.pending_question.is_none());
        assert!(s.pending_ask_user.is_none());
    }

    #[test]
    fn pending_approval_stores_tool_name() {
        let pa = PendingApproval {
            id: "approval-1".into(),
            tool_name: "Bash".into(),
            risk: Some("high".into()),
        };
        assert_eq!(pa.tool_name, "Bash");
    }
}
