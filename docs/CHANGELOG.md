# Changelog

All notable changes to ReflectDesktop are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project adheres to [Semantic Versioning](https://semver.org/).

## Unreleased

### Added
- **Docs**: `docs/codebase-map.md` — task-oriented "if you need X, edit Y" navigation.
- **Docs**: `docs/PROTOCOL_BRIDGE.md` — full Tauri ↔ reflect-protocol envelope spec (32 events + 15 ops, id pairing rules, version compat).
- **Docs**: `docs/assets/` + `docs/screenshots/` with placeholder SVG and capture conventions.
- **Docs**: this `CHANGELOG.md`.

### Changed
- N/A.

### Deprecated
- N/A.

### Removed
- N/A.

### Fixed
- N/A.

### Security
- N/A.

---

## 0.1.0 — 2026-07-07 (initial MVP)

First public-able milestone. App launches, three-pane layout renders, session list works, Tauri command bridge round-trips.

### Added
- **M1.1 Scaffold** (commit `2c73335`): Tauri 2 + React 19 + Vite + TS workspace; `reflect-desktop` binary name; 5 icons; capabilities.
- **M1.2 Protocol Bridge** (M1.2): 14 Tauri commands + 1 push event (`reflect_event`); `forward_agent_events` loop; 4/4 E2E tests.
- **M1.3 Three-pane Layout** (M1.3): left sidebar + chat + right panel + footer status bar; session list placeholder.
- **M1.4 Chat Render** (M1.4): `MessageList`, `MessageRow`, `Composer` — placeholder for streaming / Markdown / tool rows.
- **M1.5 Composer + Slash Popup** (M1.5): `/` popup, `slashCommands.ts` catalog.
- **M1.6 Modal Suite** (M1.6): `ModalShell` + `index.tsx` covering approval / question / plan / ask_user.
- **M1.7 Status Bar + Settings** (M1.7): top + bottom bars; `SettingsView` skeleton with Display/Editor/Provider sections.
- **M1.8 Polish** (M1.8): macOS overlay titlebar (`titleBarStyle: "Overlay"`), `macOSPrivateApi: true`, USER_GUIDE.
- **M2.x Real Backend** (commit `65bff4b`): `MinimalAgent` replaced with real `reflect_core::AgentThread` (M2.x = stub model + `EchoTool`; no network deps). Session event broadcast via `tokio::sync::broadcast` (fan-out).
- **4 product linkages**: `tray.rs`, `menu.rs`, `shortcut.rs`, `dock.rs` (macOS close-to-tray).
- **Routing** (commit `29044a4`): TanStack Router v1 + TanStack Query v5.
- **B1..B6 feature slices** (commits `d7ddb03`, `1dee7c1`, `3a85fce`, `76d7717`): Home / Threads / Models / Settings / Files / Git / Skills / Workspaces / Plan / Prompts / Notifications / Terminal views all rendered.
- **Tests** (commit `2ed587c`): Vitest config + initial tests (`SettingsView`, `MessageList`, `ThreadsView`, `useSessions`, `agent`).

### Notes
- The M2.x backend is intentionally **stubbed** (single `EchoTool`, empty `ModelRegistry`). Network-backed LLM clients and the remaining 21 builtin tools land in M3.x.
- `reflect_delete_session` returns `delete not implemented in M1; use archive in M2.4` to prevent accidental data loss.
- All events use `snake_case` discriminator on the wire (`#[serde(tag = "type", rename_all = "snake_case")]`); Rust `PascalCase` struct variants are an internal detail.

---

## Versioning Policy

- **Major**: protocol-level breaking changes (`Op` / `EventMsg` shape changes).
- **Minor**: new feature slices, new Tauri commands, new event variants (always **additive**).
- **Patch**: bug fixes, doc updates, internal refactors.

GUI follows Reflect-Agent's main version cadence; minor versions may ship independently (protocol is additive-stable).