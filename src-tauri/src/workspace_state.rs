use std::path::PathBuf;

use once_cell::sync::Lazy;
use parking_lot::RwLock;

static WORKSPACE_OVERRIDE: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));

pub(crate) struct WorkspaceState;

impl WorkspaceState {
    pub(crate) fn set(path: PathBuf) {
        *WORKSPACE_OVERRIDE.write() = Some(path);
    }

    pub(crate) fn get() -> Option<PathBuf> {
        WORKSPACE_OVERRIDE.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_share_the_same_override() {
        let path = PathBuf::from("/tmp/reflect-workspace-state-test");
        WorkspaceState::set(path.clone());
        assert_eq!(WorkspaceState::get(), Some(path));
    }
}
