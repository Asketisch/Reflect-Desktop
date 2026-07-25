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
    _policy: &reflect_protocol::ApprovalPolicy,
) -> crate::state::ApprovalPolicy {
    // reflect-protocol uses stringly-typed policy; for now, default to Auto.
    // Real mapping can read a field if needed.
    crate::state::ApprovalPolicy::Auto
}

pub(super) fn map_sandbox_policy(
    _policy: &reflect_protocol::SandboxPolicy,
) -> crate::state::SandboxPolicy {
    crate::state::SandboxPolicy::WorkspaceOnly
}

pub(super) fn map_permission_mode(
    policy: &reflect_protocol::ApprovalPolicy,
) -> reflect_protocol::PermissionMode {
    // ApprovalPolicy is snake_case stringly; PermissionMode has more variants.
    // For Phase 1, return Auto; future work reads the actual value.
    let _ = policy;
    reflect_protocol::PermissionMode::Auto
}
