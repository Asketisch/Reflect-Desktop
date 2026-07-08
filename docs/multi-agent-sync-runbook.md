# Multi-Agent / Vendor Sync Runbook

> Canonical checklist for syncing `vendor/reflect-*` from upstream `Reflect-Agent`. Mirrors CodexMonitor's `docs/multi-agent-sync-runbook.md`.

## When to sync

- Reflect-Agent publishes a new release (tag `v*`).
- A bug fix lands in `reflect-protocol` / `reflect-core` / `reflect-rollout` / etc. that affects GUI behavior.
- A new `Op` / `EventMsg` variant is added (frontend types need regeneration).

## Pre-flight

1. Confirm the upstream commit is on a branch we want to track (`main` by default).
2. Run the local test suite **before** the sync to record the baseline:

   ```bash
   pnpm test
   cd src-tauri && cargo test --workspace
   ```

## Sync procedure

```bash
# 1. Pull latest from a local clone or fork
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
# OR supply a tag
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent v1.2.0

# 2. Review the diff
git diff --stat vendor/
git diff vendor/reflect-protocol/src/

# 3. Regenerate frontend types if reflect-protocol changed
cd /path/to/Reflect-Agent
cargo run -p reflect-protocol --example dump_schema > /tmp/reflect-schema.json
cd /Users/admin/Code/CNB/ReflectDesktop
npx json2ts /tmp/reflect-schema.json -o src/types/protocol.ts

# 4. Rebuild
pnpm install
cd src-tauri && cargo check
pnpm test
pnpm typecheck
pnpm tauri build --bundles app

# 5. Commit
git add vendor/ src/types/protocol.ts
git commit -m "sync vendor: <reason> (Reflect-Agent <tag>)"
```

## Conflict policy

- **vendor/**: never edit directly. If a hot-fix is needed, patch upstream first, then re-sync.
- **src/types/protocol.ts**: regenerate from schema; do not hand-edit unless absolutely necessary.
- **app-core/**: shared between GUI and Tauri shim; coordinate with upstream if changing.

## What can break

| Symptom | Cause | Fix |
|---|---|---|
| `cargo check` fails with "missing field" | `reflect-protocol` added a new variant | Update `src/types/protocol.ts` via `json2ts` |
| Frontend discards events | Frontend reducer lacks new variant | Add `case` to `src/services/agent.ts::handle_event` |
| `invoke('reflect_submit')` errors | Tauri command signature drift | Sync `src-tauri/src/commands/mod.rs` with new `Op` variants |
| Binary size jumps | New vendored crate pulled in | Check `scripts/vendor-sync.sh` `CRATES` list |

## Rollback

If the sync breaks the GUI:

```bash
git revert <sync-commit-sha>
pnpm install
cd src-tauri && cargo check
```

Then file an issue upstream describing the breakage.

## CRATES list

`scripts/vendor-sync.sh` controls which crates get mirrored. To add or remove a crate, edit the `CRATES=(...)` array at the top of the script and rerun the sync. The full current set is documented in [`codebase-map.md`](codebase-map.md#vendor-crates).

## Related docs

- Architecture (vendor mirror rationale): [`ARCHITECTURE.md`](ARCHITECTURE.md#5-vendor-mirror)
- Codebase map (paths): [`codebase-map.md`](codebase-map.md)
- Protocol envelope: [`PROTOCOL_BRIDGE.md`](PROTOCOL_BRIDGE.md)