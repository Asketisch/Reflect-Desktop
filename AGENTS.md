# ReflectDesktop Agent Guide

All docs must be canonical, with no past commentary, only live state.

## Scope

This file is the agent contract for how to work in this repo.
Detailed navigation/runbooks live in:

- `docs/codebase-map.md` (task-oriented file map: "if you need X, edit Y")
- `docs/PROTOCOL_BRIDGE.md` (Tauri ↔ reflect-protocol envelope spec)
- `README.md` (setup, build, release, and broader project docs)
- `docs/CHANGELOG.md` (release notes)

## Project Snapshot

ReflectDesktop is a Tauri 2 + React 19 desktop GUI for the Reflect Agent.

- Frontend: React 19 + Vite + TanStack Router/Query (`src/`)
- Backend app: Tauri Rust process (`src-tauri/src/lib.rs`)
- Vendor mirror: read-only mirror of `reflect-*` crates under `vendor/`, kept in sync via `scripts/vendor-sync.sh`
- Shared app-core: `app-core/` (UI-agnostic reducer/state, shared with future Tauri shims)

## Non-Negotiable Architecture Rules

1. Put shared/domain backend logic in `app-core/` and `vendor/reflect-*` first.
2. The Tauri app is a thin adapter around `reflect_core::AgentThread`.
3. Do not duplicate logic between TUI and GUI; both consume the same `reflect-protocol` envelope.
4. Keep Tauri command names and payload shapes stable unless intentionally changing contracts.
5. Keep frontend IPC contracts in sync with backend command surfaces (`src/utils/tauri.ts` ↔ `src-tauri/src/commands/mod.rs`).

## Backend Routing Rules

For backend behavior changes, follow this order:

1. Vendor crates (`vendor/reflect-*`) — preferred for cross-runtime logic. If the change belongs upstream, sync via `scripts/vendor-sync.sh` first.
2. App adapter and Tauri command surface (`src-tauri/src/lib.rs` + `commands/mod.rs`).
3. Frontend IPC wrapper (`src/utils/tauri.ts`).
4. If you add a backend command, update all relevant layers + tests + `docs/PROTOCOL_BRIDGE.md` + `docs/CHANGELOG.md`.

## Frontend Routing Rules

- Keep `src/App.tsx` as composition/wiring root.
- Keep `src/router.tsx` as the route table.
- Move stateful orchestration into `src/features/<slice>/hooks/*`.
- Keep presentational UI in feature components (`src/features/<slice>/<View>.tsx`).
- Keep Tauri calls in `src/utils/tauri.ts` only.
- Keep event subscription fanout in `src/services/agent.ts`.

## Import Aliases

Use project aliases for frontend imports (defined in `tsconfig.json` + `vite.config.ts`):

- `@/*` → `src/*`

## Key File Anchors

- Frontend composition root: `src/main.tsx`
- Frontend router: `src/router.tsx`
- IDE app shell: `src/features/shell/AppShell.tsx`
- Frontend IPC wrapper: `src/utils/tauri.ts`
- Frontend agent store: `src/stores/agentStore.ts`
- Frontend agent hook (re-export): `src/services/agent.ts`
- App command registry: `src-tauri/src/lib.rs`
- App state (AgentThread host): `src-tauri/src/state.rs`
- Event forwarder: `src-tauri/src/events.rs`
- Tauri commands: `src-tauri/src/commands/mod.rs`
- Cargo workspace: `Cargo.toml` (root) + `src-tauri/Cargo.toml`
- Vendor mirror: `vendor/reflect-*/` (mirror, do not edit directly)

For broader path maps, use `docs/codebase-map.md`.

## Protocol Invariants

- Wire format is `reflect_protocol::Event` / `Submission` (snake_case JSON).
- All `Op` variants → Tauri commands (see `docs/PROTOCOL_BRIDGE.md` §2).
- All `EventMsg` variants → single `reflect_event` channel; frontend dispatches by `msg.type`.
- Submission id ↔ Event id pairing for stream correlation.
- `EVENT_ID_NONE = ""` for lifecycle events (no matching submission).

## AgentThread Host State Invariants

- `MinimalAgent` (M2.x) is the wrapper that holds `Arc<AgentThread>` + `tokio::sync::broadcast` for session event fan-out.
- `install_agent_thread()` is called from Tauri `setup()` (post runtime init) — never from `manage()` (sync phase).
- Slow subscribers trigger `RecvError::Lagged(n)` — logged but task keeps running.
- macOS close button → hide to tray (do not exit) via `on_window_event`.

## Session Hierarchy Invariants

- `useSessions` (`src/features/sessions/hooks/useSessions.ts`) is the canonical hook for session list.
- Switching sessions → `reflect_replay_session(id)` then hydrate local state.
- Time bucketing (`Now/Today/Yesterday/ThisWeek/Older`) lives in the hook, not the sidebar component.

## Follow-up Behavior Map

For Queue vs Steer follow-up behavior, start here:

- Settings model + defaults: `src/features/settings/SettingsView.tsx`
- Composer runtime behavior: `src/features/messages/Composer.tsx`
- Send intent routing: `src/stores/agentStore.ts::submit`
- App/layout wiring: `src/features/shell/AppShell.tsx`, `src/router.tsx`

## App State Sync Checklist

When changing settings/persistence that affects both backend and frontend:

1. Backend (`src-tauri/src/state.rs` + `commands/mod.rs`) updated.
2. Frontend IPC (`src/utils/tauri.ts`) updated.
3. Feature settings UI (`src/features/settings/SettingsView.tsx`) updated.
4. Test coverage added.
5. `docs/PROTOCOL_BRIDGE.md` updated if wire format changed.
6. `docs/CHANGELOG.md` entry added.

## Design System Rule (High-Level)

Use existing design tokens (`src/styles/tokens.css`, dark-first with light/system themes) and primitives (`src/features/design-system/primitives/*`) for shared shell chrome. Do not reintroduce duplicated modal/toast/panel/popover shell styling in feature CSS. All views consume `token` via CSS Modules (no inline style hardcoded colors).

(See existing DS files and `DesignSystemView` catalog for implementation details.)

## Safety and Git Behavior

- Prefer safe git operations (`status`, `diff`, `log`).
- Do not reset/revert unrelated user changes.
- If unrelated changes appear, continue focusing on owned files unless they block correctness.
- If conflicts impact correctness, call them out and choose the safest path.
- Fix root cause, not band-aids.
- **Vendor mirror**: never edit `vendor/reflect-*` directly — use `scripts/vendor-sync.sh`.

## Validation Matrix

Run validations based on touched areas:

- Always: `pnpm typecheck`
- Frontend behavior/state/hooks/components: `pnpm test`
- Rust backend changes: `cd src-tauri && cargo check`
- Use targeted tests for touched modules before full-suite runs when iterating.

## Quick Runbook

Core local commands (keep these inline for daily use):

```bash
pnpm install
pnpm tauri dev                # dev mode (HMR)
pnpm test                     # vitest
pnpm typecheck                # tsc --noEmit
cd src-tauri && cargo check   # Rust types
```

Release build:

```bash
pnpm tauri build              # full multi-platform
pnpm tauri build --bundles app  # macOS .app only
bash scripts/install.sh       # → /usr/local/bin/reflect-desktop + ~/Applications/ReflectDesktop.app
```

Vendor sync (when Reflect-Agent exports a new release):

```bash
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
git diff --stat vendor/
git add vendor/ && git commit -m "sync vendor: <reason>"
```

Focused test runs:

```bash
pnpm test -- src/features/settings/SettingsView.test.tsx
```

## Hotspots

Use extra care in high-churn/high-complexity files:

- `src-tauri/src/lib.rs` (Tauri builder + 20 commands registration)
- `src-tauri/src/state.rs` (AgentThread host; install timing matters)
- `src-tauri/src/commands/mod.rs` (19 commands + session I/O)
- `src-tauri/src/events.rs` (event forwarder, single channel)
- `src/stores/agentStore.ts` (single source of truth for agent state + reducer)
- `src/features/shell/AppShell.tsx` (IDE 5-pane layout; sidebar/inspector toggle, session routing)
- `src/features/messages/MessageList.tsx` (chat rendering + Collapsible items)
- `src/utils/tauri.ts` (barrel; the only file that re-exports `invoke` / `listen` from `commands/bridge`)
- `src/router.tsx` (TanStack Router route table)
- `src/styles/tokens.css` (design tokens; dark/light/system themes — single source of truth)
- `scripts/vendor-sync.sh` (CRATES list controls which vendor crates sync)

## Canonical References

- Task-oriented code map: `docs/codebase-map.md`
- Protocol envelope: `docs/PROTOCOL_BRIDGE.md`
- Architecture: `docs/ARCHITECTURE.md`
- Setup/build/release/test commands: `README.md`
- Change log: `docs/CHANGELOG.md`
- GUI design history: `docs/gui/00-index.md`