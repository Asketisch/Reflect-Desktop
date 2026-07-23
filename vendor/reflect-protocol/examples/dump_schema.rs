//! Dump a JSON Schema for the protocol's top-level types to stdout.
//!
//! **B1-01 alignment**: this now uses `schemars` derives on the actual
//! `EventMsg` / `Op` / `Submission` / `UserInputItem` / `ContentBlock` /
//! nested payload types — no hand-written variant lists. Adding a new
//! `EventMsg` variant in `event_msg.rs` automatically appears in the
//! generated schema (and in the downstream TypeScript types after json2ts).
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p reflect-protocol --example dump_schema > /tmp/reflect-schema.json
//! npx json2ts /tmp/reflect-schema.json -o src/types/protocol.ts
//! ```
//!
//! Or use the wrapper script:
//!
//! ```bash
//! bash scripts/dump-ts-types.sh
//! ```

use schemars::schema_for;
use serde_json::{json, Value};

use reflect_protocol::{EventMsg, Op, Submission};

fn main() {
    // Top-level EventMsg schema: derive directly from the protocol type.
    let event_msg_schema = schema_for!(EventMsg);
    let op_schema = schema_for!(Op);
    let submission_schema = schema_for!(Submission);

    // Outer wire envelope: `{ id: string, msg: EventMsg }`. This is the
    // Tauri event payload that the frontend's `onReflectEvent` listener
    // receives. Mirroring it in JSON Schema lets json2ts produce a matching
    // `ReflectEvent` TS type.
    let event_envelope = json!({
        "type": "object",
        "required": ["id", "msg"],
        "properties": {
            "id": { "type": "string" },
            "msg": event_msg_schema,
        }
    });

    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "ReflectProtocol",
        "definitions": {
            "Submission": submission_schema,
            "Event": event_envelope,
            "Op": op_schema,
        }
    });

    // Strip top-level `$defs` (schemars uses this for type reuse, json2ts
    // can choke on it). Keep our explicit `definitions` block intact.
    let cleaned = strip_dollar_defs(schema);

    println!(
        "{}",
        serde_json::to_string_pretty(&cleaned).expect("serialize schema")
    );
}

/// Recursively remove `$defs` keys (schemars type-reuse container) so the
/// generated TypeScript surface is clean for json2ts. Keeps our explicit
/// `definitions` block (the four entry points: Submission / Event / Op / etc).
fn strip_dollar_defs(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("$defs");
        // Recurse into nested values.
        for (_k, val) in obj.iter_mut() {
            *val = strip_dollar_defs(std::mem::take(val));
        }
    }
    v
}
