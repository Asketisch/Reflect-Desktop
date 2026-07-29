use reflect_protocol::TurnId;

use crate::state::{RenderState, ServerState, ServerStatus, Turn};

pub(super) fn turn_mut(state: &mut RenderState, turn_id: Option<TurnId>) -> Option<&mut Turn> {
    let turn_id = turn_id?;
    state.turns.iter_mut().find(|turn| turn.id == turn_id)
}

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

pub(super) fn map_approval_policy(
    policy: &reflect_protocol::ApprovalPolicy,
) -> crate::state::ApprovalPolicy {
    match policy {
        reflect_protocol::ApprovalPolicy::Auto => crate::state::ApprovalPolicy::Auto,
        reflect_protocol::ApprovalPolicy::Prompt => crate::state::ApprovalPolicy::Prompt,
        reflect_protocol::ApprovalPolicy::Deny => crate::state::ApprovalPolicy::Deny,
    }
}

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

pub(super) fn map_permission_mode(
    policy: &reflect_protocol::ApprovalPolicy,
) -> reflect_protocol::PermissionMode {
    // SessionConfiguredEvent only carries ApprovalPolicy; map the overlapping
    // variants directly. Variants exclusive to PermissionMode (Plan /
    // AcceptEdits / Bubble / Bypass) cannot be derived from an ApprovalPolicy
    // and fall back to Auto.
    match policy {
        reflect_protocol::ApprovalPolicy::Auto => reflect_protocol::PermissionMode::Auto,
        reflect_protocol::ApprovalPolicy::Prompt => reflect_protocol::PermissionMode::Prompt,
        reflect_protocol::ApprovalPolicy::Deny => reflect_protocol::PermissionMode::Deny,
    }
}
