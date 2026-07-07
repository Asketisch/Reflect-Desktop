//! `reflect-async-graph` — v2.0 异步流式图引擎骨架。
//!
//! 目标:动态节点、条件边、可视化 DAG。当前仅 trait + 内存图 stub,
//! 不替代 `reflect-core` 现有 4-node StateGraph。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// 图节点 id。
pub type NodeId = String;

/// 节点执行上下文( opaque JSON state )。
pub type GraphState = serde_json::Value;

/// 单节点执行结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeOutput {
    pub node_id: NodeId,
    pub next: Option<NodeId>,
    pub state_patch: GraphState,
}

/// 动态图节点 trait。
#[async_trait]
pub trait AsyncGraphNode: Send + Sync {
    fn id(&self) -> &str;
    async fn run(&self, state: GraphState) -> anyhow::Result<NodeOutput>;
}

/// 条件边:根据 state 选择下一节点。
pub type EdgeFn = Arc<dyn Fn(&GraphState) -> Option<NodeId> + Send + Sync>;

/// 异步流式图 —— 骨架实现(顺序执行,无并行 fan-out)。
pub struct AsyncGraph {
    nodes: HashMap<NodeId, Arc<dyn AsyncGraphNode>>,
    edges: HashMap<NodeId, EdgeFn>,
    entry: NodeId,
}

impl AsyncGraph {
    pub fn new(entry: impl Into<NodeId>) -> Self {
        Self {
            nodes: HashMap::new(),
            entry: entry.into(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Arc<dyn AsyncGraphNode>, edge: Option<EdgeFn>) {
        let id = node.id().to_string();
        if let Some(e) = edge {
            self.edges.insert(id.clone(), e);
        }
        self.nodes.insert(id, node);
    }

    /// 从 entry 顺序跑至无 next 或 max_steps。
    pub async fn run(&self, mut state: GraphState, max_steps: usize) -> anyhow::Result<GraphState> {
        let mut current = Some(self.entry.clone());
        for _ in 0..max_steps {
            let Some(node_id) = current.take() else {
                break;
            };
            let node = self
                .nodes
                .get(&node_id)
                .ok_or_else(|| anyhow::anyhow!("unknown node {node_id}"))?;
            let out = node.run(state.clone()).await?;
            if let Some(patch) = out.state_patch.as_object()
                && let Some(base) = state.as_object_mut()
            {
                for (k, v) in patch {
                    base.insert(k.clone(), v.clone());
                }
            }
            current = out
                .next
                .or_else(|| self.edges.get(&node_id).and_then(|f| f(&state)));
        }
        Ok(state)
    }

    /// 流式 event 通道 stub —— 每跑完一节点推 `NodeOutput`。
    pub async fn run_stream(
        &self,
        state: GraphState,
        max_steps: usize,
    ) -> (
        mpsc::Receiver<NodeOutput>,
        tokio::task::JoinHandle<anyhow::Result<GraphState>>,
    ) {
        let (tx, rx) = mpsc::channel(32);
        let nodes = self.nodes.clone();
        let edges = self.edges.clone();
        let entry = self.entry.clone();
        let handle = tokio::spawn(async move {
            let mut current = Some(entry);
            let mut state = state;
            for _ in 0..max_steps {
                let Some(node_id) = current.take() else {
                    break;
                };
                let node = nodes
                    .get(&node_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown node {node_id}"))?;
                let out = node.run(state.clone()).await?;
                let _ = tx.send(out.clone()).await;
                if let Some(patch) = out.state_patch.as_object()
                    && let Some(base) = state.as_object_mut()
                {
                    for (k, v) in patch {
                        base.insert(k.clone(), v.clone());
                    }
                }
                current = out
                    .next
                    .or_else(|| edges.get(&node_id).and_then(|f| f(&state)));
            }
            Ok(state)
        });
        (rx, handle)
    }
}

/// Echo 节点 stub(测试用)。
pub struct EchoNode {
    id: NodeId,
    next: Option<NodeId>,
}

impl EchoNode {
    pub fn new(id: impl Into<NodeId>, next: Option<NodeId>) -> Self {
        Self {
            id: id.into(),
            next,
        }
    }
}

#[async_trait]
impl AsyncGraphNode for EchoNode {
    fn id(&self) -> &str {
        &self.id
    }

    async fn run(&self, state: GraphState) -> anyhow::Result<NodeOutput> {
        Ok(NodeOutput {
            node_id: self.id.clone(),
            next: self.next.clone(),
            state_patch: state,
        })
    }
}

/// 可视化调试 stub —— 导出 Mermaid DAG 文本。
pub fn to_mermaid_stub(graph_name: &str, node_ids: &[&str]) -> String {
    let mut s = format!("graph TD\n  subgraph {graph_name}\n");
    for (i, id) in node_ids.iter().enumerate() {
        s.push_str(&format!("    {id}[{id}]\n"));
        if i > 0 {
            s.push_str(&format!("    {} --> {}\n", node_ids[i - 1], id));
        }
    }
    s.push_str("  end\n");
    s
}

/// 供 v2 visual-debugger 使用的占位 future 类型别名。
pub type VisualDebugHook = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn echo_graph_runs() {
        let mut g = AsyncGraph::new("a");
        g.add_node(Arc::new(EchoNode::new("a", Some("b".into()))), None);
        g.add_node(Arc::new(EchoNode::new("b", None)), None);
        let out = g.run(json!({"x": 1}), 8).await.unwrap();
        assert_eq!(out["x"], 1);
    }

    #[test]
    fn mermaid_stub() {
        let m = to_mermaid_stub("test", &["a", "b"]);
        assert!(m.contains("a --> b"));
    }
}
