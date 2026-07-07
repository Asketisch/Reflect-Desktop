//! `StateGraph` — the 4-node agent loop driver (M2/M3).
//!
//! Walks `PreLoop → ModelCall → ToolExec → CheckStop → (PreLoop | end)`.
//! M3 wires hooks via `nodes::check_stop` and the `ToolExecutionQueue`.

pub mod nodes;
pub mod state;

pub use state::AgentState;

use reflect_llm::SharedModelRegistry;
use reflect_protocol::TurnId;
use reflect_tools::ToolExecutionQueue;
use std::sync::Arc;

use crate::submission_loop::NodeContext;

/// The four nodes of the v0 agent loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNode {
    /// `pre_loop` — microcompact + reminder injection + skills activation.
    PreLoop,
    /// `model_call` — LLM call + prompt caching injection.
    ModelCall,
    /// `tool_exec` — call `ToolExecutionQueue` for any tool_use blocks.
    ToolExec,
    /// `check_stop` — Stop hook + max-iteration safety valve.
    CheckStop,
}

impl GraphNode {
    /// Default transition: pre_loop → model_call → tool_exec → check_stop → model_call
    /// (loops on tool_exec if the LLM returned tool_use blocks; M2).
    pub fn next(self, has_tool_calls: bool) -> Option<GraphNode> {
        match (self, has_tool_calls) {
            (GraphNode::PreLoop, _) => Some(GraphNode::ModelCall),
            (GraphNode::ModelCall, true) => Some(GraphNode::ToolExec),
            (GraphNode::ModelCall, false) => Some(GraphNode::CheckStop),
            (GraphNode::ToolExec, _) => Some(GraphNode::CheckStop),
            (GraphNode::CheckStop, true) => Some(GraphNode::PreLoop),
            (GraphNode::CheckStop, false) => None, // turn complete
        }
    }
}

pub struct StateGraph {
    pub state: AgentState,
    pub ctx: NodeContext,
}

impl StateGraph {
    pub fn new(state: AgentState, ctx: NodeContext) -> Self {
        Self { state, ctx }
    }

    /// Walk the 4-node loop. Returns the final `AgentState`.
    /// Sets `state.completed_normally = true` only when the loop ends at
    /// `CheckStop` (natural end of turn), not when `model_call` aborts
    /// due to error / cancel.
    pub async fn run(mut self) -> AgentState {
        let mut node = GraphNode::PreLoop;
        loop {
            let next = match node {
                GraphNode::PreLoop => nodes::pre_loop(&mut self.state, &self.ctx).await,
                GraphNode::ModelCall => nodes::model_call(&mut self.state, &self.ctx).await,
                GraphNode::ToolExec => nodes::tool_exec(&mut self.state, &self.ctx).await,
                GraphNode::CheckStop => nodes::check_stop(&mut self.state, &self.ctx).await,
            };
            let Some(next) = next else {
                // Natural termination comes from check_stop returning None
                // (= allow). If model_call or tool_exec returned None, the
                // turn ended abnormally; do not mark completed_normally.
                if matches!(node, GraphNode::CheckStop) {
                    self.state.completed_normally = true;
                }
                break;
            };
            node = next;
        }
        self.state
    }
}

#[allow(dead_code)]
fn _touch_unused(_r: SharedModelRegistry, _t: Arc<ToolExecutionQueue>, _id: TurnId) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_loop_always_goes_to_model_call() {
        assert_eq!(GraphNode::PreLoop.next(false), Some(GraphNode::ModelCall));
        assert_eq!(GraphNode::PreLoop.next(true), Some(GraphNode::ModelCall));
    }

    #[test]
    fn model_call_branches_on_tool_use() {
        assert_eq!(GraphNode::ModelCall.next(false), Some(GraphNode::CheckStop));
        assert_eq!(GraphNode::ModelCall.next(true), Some(GraphNode::ToolExec));
    }

    #[test]
    fn tool_exec_always_goes_to_check_stop() {
        assert_eq!(GraphNode::ToolExec.next(false), Some(GraphNode::CheckStop));
        assert_eq!(GraphNode::ToolExec.next(true), Some(GraphNode::CheckStop));
    }

    #[test]
    fn check_stop_loops_or_terminates() {
        assert_eq!(GraphNode::CheckStop.next(true), Some(GraphNode::PreLoop));
        assert_eq!(GraphNode::CheckStop.next(false), None);
    }

    #[test]
    fn full_walk_terminates_after_no_tool_calls() {
        let mut node = GraphNode::PreLoop;
        let mut path = vec![node];
        while let Some(next) = node.next(false) {
            path.push(next);
            node = next;
        }
        assert_eq!(
            path,
            vec![
                GraphNode::PreLoop,
                GraphNode::ModelCall,
                GraphNode::CheckStop
            ]
        );
    }
}
