//! `ForkedSession` — a thin handle for tracking parent / child relationships
//! in the rollout JSONL.

use reflect_protocol::RolloutRecord;
use reflect_protocol::ThreadId;

/// Returned by `SubAgentFactory::spawn` so the parent can later correlate
/// child events with the parent's record of having forked a sub-session.
#[derive(Debug, Clone)]
pub struct ForkedSession {
    pub session_id: ThreadId,
    pub parent_session_id: ThreadId,
    pub branch_name: String,
}

impl ForkedSession {
    /// Build the [`RolloutRecord::Fork`] that should be appended to the
    /// parent's rollout file at the moment of the fork.
    pub fn into_fork_record(self) -> RolloutRecord {
        RolloutRecord::Fork {
            parent_session_id: self.parent_session_id,
            branch_name: self.branch_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_fork_record_preserves_fields() {
        let parent = ThreadId::new();
        let child = ThreadId::new();
        let f = ForkedSession {
            session_id: child,
            parent_session_id: parent,
            branch_name: "explorer".into(),
        };
        let r = f.into_fork_record();
        match r {
            RolloutRecord::Fork {
                parent_session_id,
                branch_name,
            } => {
                assert_eq!(parent_session_id, parent);
                assert_eq!(branch_name, "explorer");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}
