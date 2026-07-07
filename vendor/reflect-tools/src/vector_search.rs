//! `vector_search` —— 向量语义搜索 stub(P2 `vector-search`)。
//!
//! v2 接入真实 embedding 模型;当前用 token hash 向量 + 余弦相似度。

use serde::{Deserialize, Serialize};

/// 单条可检索文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

/// 检索命中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub id: String,
    pub score: f32,
    pub snippet: String,
}

/// 内存向量索引 stub。
#[derive(Debug, Default)]
pub struct VectorIndex {
    docs: Vec<VectorDocument>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入文档并自动生成 stub embedding。
    pub fn insert(&mut self, id: impl Into<String>, text: impl Into<String>) {
        let text = text.into();
        let embedding = stub_embed(&text);
        self.docs.push(VectorDocument {
            id: id.into(),
            text,
            embedding,
        });
    }

    /// 余弦相似度 top-k 检索。
    pub fn search(&self, query: &str, k: usize) -> Vec<VectorHit> {
        let q = stub_embed(query);
        let mut scored: Vec<VectorHit> = self
            .docs
            .iter()
            .map(|d| VectorHit {
                id: d.id.clone(),
                score: cosine(&q, &d.embedding),
                snippet: d.text.chars().take(120).collect(),
            })
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored.truncate(k);
        scored
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

/// stub embedding:固定维度 bag-of-token hash。
pub fn stub_embed(text: &str) -> Vec<f32> {
    const DIM: usize = 32;
    let mut v = vec![0f32; DIM];
    for tok in text.split_whitespace() {
        let h = fnv1a(tok.as_bytes()) as usize;
        v[h % DIM] += 1.0;
    }
    let norm = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 2166136261;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ranks_relevant_doc_higher() {
        let mut idx = VectorIndex::new();
        idx.insert("a", "rust ownership borrow checker");
        idx.insert("b", "python pandas dataframe tutorial");
        let hits = idx.search("rust borrow", 2);
        assert_eq!(hits[0].id, "a");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn stub_embed_is_normalized() {
        let v = stub_embed("hello world");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5 || norm == 0.0);
    }
}
