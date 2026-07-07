//! Hashline 编辑 —— 用 content-hash 锚定行块,避免 LLM 漂移。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 单行锚: `{hash}:{line_no}:{text}`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashlineAnchor {
    pub hash: String,
    pub line_no: usize,
    pub text: String,
}

/// 对单行文本计算短 hash(前 8 位 hex)。
pub fn line_hash(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:08x}", hasher.finish())[..8].to_string()
}

/// 解析 `hash:line:text` 锚字符串。
pub fn parse_anchor(s: &str) -> Option<HashlineAnchor> {
    let mut parts = s.splitn(3, ':');
    let hash = parts.next()?.to_string();
    let line_no: usize = parts.next()?.parse().ok()?;
    let text = parts.next()?.to_string();
    Some(HashlineAnchor {
        hash,
        line_no,
        text,
    })
}

/// 在文件内容中定位锚;hash 或行号不匹配时返回 `None`。
pub fn locate_anchor(content: &str, anchor: &HashlineAnchor) -> Option<(usize, usize)> {
    for (i, line) in content.lines().enumerate() {
        let n = i + 1;
        if n == anchor.line_no && line_hash(line) == anchor.hash && line == anchor.text {
            let start = content.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
            let end = start + line.len();
            return Some((start, end));
        }
    }
    None
}

/// 用 `new_text` 替换锚定行;失败返回原内容。
pub fn apply_hashline_replace(content: &str, anchor: &HashlineAnchor, new_text: &str) -> String {
    let Some((start, end)) = locate_anchor(content, anchor) else {
        return content.to_string();
    };
    let mut out = String::with_capacity(content.len() + new_text.len());
    out.push_str(&content[..start]);
    out.push_str(new_text);
    out.push_str(&content[end..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_and_replace() {
        let content = "fn main() {\n    println!(\"hi\");\n}\n";
        let line = "    println!(\"hi\");";
        let anchor = HashlineAnchor {
            hash: line_hash(line),
            line_no: 2,
            text: line.to_string(),
        };
        let next = apply_hashline_replace(content, &anchor, "    println!(\"bye\");");
        assert!(next.contains("bye"));
        assert!(!next.contains("\"hi\""));
    }
}
