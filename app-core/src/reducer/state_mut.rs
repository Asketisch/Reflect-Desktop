//! RenderState 可变辅助函数 —— 供 matchers 调用的内部工具。
//!
//! 包含 Turn 查找、Server upsert、以及协议层枚举到 RenderState 枚举的映射函数。

use reflect_protocol::TurnId;

use crate::state::{RenderState, ServerState, ServerStatus, Turn};

/// 按 TurnId 查找可变的 Turn 引用（turn_id 为 None 时返回 None）。
pub(super) fn turn_mut(state: &mut RenderState, turn_id: Option<TurnId>) -> Option<&mut Turn> {
    let turn_id = turn_id?;
    state.turns.iter_mut().find(|turn| turn.id == turn_id)
}

/// 更新或插入 ServerState：按名称查找，存在则更新，不存在则追加。
pub(super) fn upsert_server(
    list: &mut Vec<ServerState>,
    name: &str,
    status: ServerStatus,
    detail: Option<String>,
) {
    if let Some(server) = list.iter_mut().find(|server| server.name == name) {
        server.status = status;
        server.detail = detail;
    } else {
        list.push(ServerState {
            name: name.to_string(),
            status,
            detail,
        });
    }
}

/// 将协议层 ApprovalPolicy 映射为 RenderState 内部的 ApprovalPolicy。
pub(super) fn map_approval_policy(
    policy: &reflect_protocol::ApprovalPolicy,
) -> crate::state::ApprovalPolicy {
    match policy {
        reflect_protocol::ApprovalPolicy::Auto => crate::state::ApprovalPolicy::Auto,
        reflect_protocol::ApprovalPolicy::Prompt => crate::state::ApprovalPolicy::Prompt,
        reflect_protocol::ApprovalPolicy::Deny => crate::state::ApprovalPolicy::Deny,
    }
}

/// 将协议层 SandboxPolicy 映射为 RenderState 内部的 SandboxPolicy。
pub(super) fn map_sandbox_policy(
    policy: &reflect_protocol::SandboxPolicy,
) -> crate::state::SandboxPolicy {
    match policy {
        reflect_protocol::SandboxPolicy::WorkspaceOnly => {
            crate::state::SandboxPolicy::WorkspaceOnly
        }
        reflect_protocol::SandboxPolicy::OsSandbox => crate::state::SandboxPolicy::OsSandbox,
        reflect_protocol::SandboxPolicy::FullAccess => crate::state::SandboxPolicy::FullAccess,
    }
}

/// 将协议层 ApprovalPolicy 映射为 PermissionMode。
///
/// SessionConfiguredEvent 仅携带 ApprovalPolicy；直接将交集变体一一对应。
/// PermissionMode 独占的变体（Plan / AcceptEdits / Bubble / Bypass）
/// 无法从 ApprovalPolicy 推导，回退为 Auto。
pub(super) fn map_permission_mode(
    policy: &reflect_protocol::ApprovalPolicy,
) -> reflect_protocol::PermissionMode {
    match policy {
        reflect_protocol::ApprovalPolicy::Auto => reflect_protocol::PermissionMode::Auto,
        reflect_protocol::ApprovalPolicy::Prompt => reflect_protocol::PermissionMode::Prompt,
        reflect_protocol::ApprovalPolicy::Deny => reflect_protocol::PermissionMode::Deny,
    }
}
