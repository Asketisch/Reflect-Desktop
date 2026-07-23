#!/usr/bin/env bash
# dump-ts-types.sh — regenerate `src/types/protocol.ts` from the Rust
# reflect-protocol types via JSON Schema.
#
# Pipeline:
#   1. cargo run -p reflect-protocol --example dump_schema
#        → JSON Schema (using schemars JsonSchema derives on protocol types)
#   2. npx json2ts /tmp/reflect-schema.json
#        → src/types/protocol.ts
#
# Why this matters (B1-01):
#   Before, the protocol.ts was a hand-maintained 33-line summary that
#   diverged from the Rust side on every `Op` / `EventMsg` addition. Now
#   any new `Op::Foo` variant in vendor/reflect-protocol/src/op.rs
#   automatically shows up in the TS type after running this script.
#
# Usage:
#   bash scripts/dump-ts-types.sh
#   pnpm run types:gen
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCHEMA_TMP="/tmp/reflect-schema.json"
TS_OUT="src/types/protocol.generated.ts"

echo "→ generating JSON Schema from reflect-protocol..."
cargo run --quiet -p reflect-protocol --example dump_schema 2>/dev/null > "$SCHEMA_TMP"
echo "  schema: $SCHEMA_TMP ($(wc -l < "$SCHEMA_TMP") lines, $(stat -f%z "$SCHEMA_TMP" 2>/dev/null || stat -c%s "$SCHEMA_TMP") bytes)"

# json2ts requires network. If unavailable, fall back to leaving the
# manually-maintained src/types/protocol.ts in place; the script is
# a regeneration helper, not a build dependency.
if ! command -v npx >/dev/null 2>&1; then
  echo "  ⚠ npx not found; skipping json2ts (regeneration requires Node.js)"
  exit 0
fi

echo "→ running json2ts → $TS_OUT..."
npx --yes json2ts --cwd "$REPO_ROOT" "$SCHEMA_TMP" > "$TS_OUT" 2>/dev/null || {
  echo "  ⚠ json2ts failed (offline or registry issue); $TS_OUT not written"
  exit 0
}

# Header banner so generated file is clearly marked.
cat > "${TS_OUT}.bak" <<'BANNER'
/**
 * AUTO-GENERATED — DO NOT EDIT.
 *
 * Source: vendor/reflect-protocol types via `cargo run --example dump_schema`
 * Regenerate with: `bash scripts/dump-ts-types.sh` or `pnpm run types:gen`
 */
BANNER
cat "$TS_OUT" >> "${TS_OUT}.bak"
mv "${TS_OUT}.bak" "$TS_OUT"

echo "  → wrote $TS_OUT"
echo "✓ done"
