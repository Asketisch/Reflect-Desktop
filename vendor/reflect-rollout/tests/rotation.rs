//! End-to-end test: write enough records to trigger rotation, then verify
//! the rotated file exists and the current file shrunk back to small.

use reflect_protocol::{MessageRole, RolloutRecord, RolloutRecorder, ThreadId, TurnId};
use reflect_rollout::JsonlRolloutWriter;
use tempfile::tempdir;

#[tokio::test]
async fn rotates_at_256kb_and_keeps_at_most_3_copies() {
    let dir = tempdir().unwrap();
    let sid = ThreadId::new();
    let writer = JsonlRolloutWriter::new(dir.path(), sid);

    // Each record is ~2 KiB; 200 records => ~400 KiB => at least 1 rotation.
    for i in 0..200 {
        writer
            .record(RolloutRecord::message(
                TurnId::new(),
                MessageRole::Assistant,
                serde_json::json!({"i": i, "blob": "x".repeat(2048)}),
            ))
            .await
            .unwrap();
    }

    // Walk the date dir and check: at least one `.1.jsonl`, never `.4.jsonl`.
    let mut found_one = false;
    let mut found_four = false;
    walk(dir.path(), &mut |path| {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.ends_with(".1.jsonl") {
            found_one = true;
        }
        if name.ends_with(".4.jsonl") {
            found_four = true;
        }
    });
    assert!(found_one, "expected a .1.jsonl rotated file");
    assert!(!found_four, "should never see .4.jsonl");
}

#[tokio::test]
async fn current_file_is_small_after_rotation() {
    let dir = tempdir().unwrap();
    let sid = ThreadId::new();
    let writer = JsonlRolloutWriter::new(dir.path(), sid);

    // Force multiple rotations.
    for i in 0..(5 * 200) {
        writer
            .record(RolloutRecord::message(
                TurnId::new(),
                MessageRole::Assistant,
                serde_json::json!({"i": i, "blob": "x".repeat(2048)}),
            ))
            .await
            .unwrap();
    }

    // The current (un-rotated) file should be much smaller than 256KB.
    let mut sizes = Vec::new();
    let sid_str = sid.to_string();
    walk(dir.path(), &mut |path| {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        // Current file is `<sid>.jsonl`; rotated ones are `<sid>.<n>.jsonl`.
        if name == format!("{sid_str}.jsonl") {
            sizes.push(std::fs::metadata(path).unwrap().len());
        }
    });
    assert!(!sizes.is_empty(), "expected a current file");
    let cur = *sizes.iter().max().unwrap();
    assert!(
        cur < 256 * 1024,
        "current file should be < 256 KiB after rotation, got {cur}"
    );
}

fn walk<F: FnMut(&std::path::Path)>(root: &std::path::Path, f: &mut F) {
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, f);
            } else {
                f(&p);
            }
        }
    }
}
