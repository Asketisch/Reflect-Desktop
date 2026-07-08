# ReflectDesktop

> Standalone Tauri 2 + React 19 desktop GUI for [Reflect Agent](https://github.com/CNB/Reflect-Agent).
> Zero-modification integration: this repo consumes `reflect-*` crates via `vendor/` mirror + a small IPC adapter on top of `reflect-protocol`.

[![v0.1.0](https://img.shields.io/badge/version-0.1.0-blue)]()
[![Tauri 2](https://img.shields.io/badge/Tauri-2-orange)]()
[![React 19](https://img.shields.io/badge/React-19-149eca)]()

---

## Why this repo exists

Reflect Agent's terminal-mode frontend (`reflect tui`) and the headless exec path (`reflect exec "..."`) carry the same `reflect-core::AgentThread`. A desktop GUI wants the same threads but in a Tauri-rendered window. Rather than shipping a Tauri shim inside ReflectAgent (which would couple Tauri 2 build artifacts into a headless agent tool), ReflectDesktop keeps all GUI code here and only **consumes** the agent through `vendor/reflect-*` snapshots pulled on demand.

---

## Features

### Sessions & Workspaces

- Time-bucketed session sidebar: **Now / Today / Yesterday / This week / Older** (`useSessions`).
- Resume, replay (`reflect_replay_session`), rename, archive, delete.
- Persistent composer draft per thread.

### Composer & Agent Controls

- Auto-grow textarea, IME-safe.
- `/` slash popup (47 commands from TUI), `@` file mention, drag/drop + paste images.
- Send: `Cmd+Enter` (macOS) / `Ctrl+Enter` (Linux/Windows).
- Interrupt: `Cmd+.` / `Ctrl+.` → `reflect_interrupt`.
- Model picker, effort toggle, permission mode cycle, approval rules.

### Chat Rendering

- `react-markdown` + Shiki for code blocks.
- Streaming via `AgentMessageDelta` events (single `reflect_event` channel).
- Tool rows collapse consecutive calls; `McpToolInvoked` adds `(via <server>)` badge.
- Collapsible Thinking blocks (`ThinkingDelta`).

### Approval & AskUser Modals

- **ApprovalModal**: Tool / Hook / Plan — Allow Once / Always / Deny.
- **QuestionModal**: multi-question, multi-select, "Other" custom text.
- **AskUserModal**: free-text reply.
- **PlanReadyModal**: Accept / Submit Changes with plan markdown review.

### macOS-native

- Overlay titlebar with vibrancy (`macOSPrivateApi: true`, `titleBarStyle: "Overlay"`).
- Dock badge (via `objc2`).
- Close-to-tray (window hide on close).
- Global shortcut via `tauri-plugin-global-shortcut`.

### Backend Integration

- **In-process** `Arc<AgentThread>` (no daemon) — same protocol envelope as TUI.
- 19 `#[tauri::command]`s covering all 15 `Op` variants + session I/O.
- Single push event `reflect_event` (payload = `reflect_protocol::Event`).
- `tokio::sync::broadcast` for session fan-out to multiple subscribers.

### Settings

- Display (theme: light / dark / system).
- Editor (vim / output-style toggles).
- Provider (default provider + API key placeholders).
- M2.x: Skills, Memory, Permissions, Tasks, Shortcuts (planned).

---

## Requirements

- Node.js ≥ 20 + pnpm
- Rust toolchain (stable)
- macOS: Xcode Command Line Tools
- Linux: `webkit2gtk-4.1`, `libayatana-appindicator3-dev`, `librsvg2-dev`
- Windows: WebView2 (Win10+ ships it), Microsoft C++ Build Tools

---

## Getting Started

### Install prebuilt (macOS arm64)

```bash
bash scripts/install.sh
# Installs:
#   /usr/local/bin/reflect-desktop           (binary)
#   ~/Applications/ReflectDesktop.app        (launchable bundle)
```

### From source

```bash
git clone https://github.com/CNB/ReflectDesktop.git
cd ReflectDesktop
pnpm install
bash scripts/vendor-sync.sh /path/to/Reflect-Agent main
pnpm tauri build --bundles app
bash scripts/install.sh
```

### Dev mode

```bash
pnpm tauri dev
# Frontend HMR on :5173, Tauri window opens
```

### Run validation suite

```bash
pnpm typecheck
pnpm test
cd src-tauri && cargo check
```

---

## Layout

```
ReflectDesktop/
├── vendor/         # git subtree mirror of reflect-* crates
│                    # updated via bash scripts/vendor-sync.sh
├── app-core/       # shared UI-agnostic reducer & state (was reflect-app-core)
├── src/            # React 19 + Vite + 24 feature-sliced components
├── src-tauri/      # Tauri 2 Rust backend (binary name: reflect-desktop)
├── Cargo.toml      # workspace root for vendor + app-core + src-tauri
├── docs/           # CHANGELOG · PROTOCOL_BRIDGE · codebase-map · ARCHITECTURE
│                    #   + USER_GUIDE · runbooks · gui/ (design history)
└── scripts/        # install.sh · vendor-sync.sh
```

---

## Tauri IPC Surface

19 commands + `ping` + `reflect_set_dock_badge`, all wrapping `reflect_protocol::Op`:

| Category | Commands |
|---|---|
| Health | `ping` |
| Submission | `reflect_submit`, `reflect_interrupt`, `reflect_compact`, `reflect_rewind`, `reflect_shutdown` |
| Approvals | `reflect_tool_approval`, `reflect_hook_approval`, `reflect_plan_approval` |
| Plan mode | `reflect_enter_plan_mode`, `reflect_exit_plan_mode` |
| Effort / Permission | `reflect_set_effort`, `reflect_set_permission_mode`, `reflect_cycle_permission_mode` |
| Ask user | `reflect_ask_user_question_response`, `reflect_ask_user_input_response` |
| Sessions | `reflect_list_sessions`, `reflect_rename_session`, `reflect_delete_session`, `reflect_replay_session` |
| Dock (macOS) | `reflect_set_dock_badge` |

Push events: single channel `reflect_event` carrying `reflect_protocol::Event` (32 variants, snake_case discriminator).

Full spec: [`docs/PROTOCOL_BRIDGE.md`](docs/PROTOCOL_BRIDGE.md).

---

## Roadmap

| Phase | Slice | Status |
|---|---|---|
| **M1.x** | scaffold + protocol bridge + 3-pane layout + composer + modal + statusbar | ✅ Done (commit `2c73335` … `76d7717`) |
| **M2.x** | real AgentThread backend + 4 product linkages (tray/menu/shortcut/dock) + macOS close-to-tray | ✅ Done (commit `65bff4b`) |
| **M3.x** | 22 builtin tools + Streamdown + Context Ring + multi-session LRU + Recipe + Cron + Deep Link | 🔧 In flight |
| **C1..C4** | tray polish, global hotkey, dock + badge, dual-binary install | ✅ Partial |
| **D1..D3** | remote daemon + iOS app + IM bridge + Workflow canvas | 📅 Planned |

Full history: [`docs/CHANGELOG.md`](docs/CHANGELOG.md).

---

## Sync agent sources

When Reflect-Agent exports a new release:

```bash
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
# OR supply a tag:
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent v1.2.0
```

This is the only sanctioned way to refresh vendor. Editing files under `vendor/` directly is not allowed (next sync will overwrite).

Full runbook: [`docs/multi-agent-sync-runbook.md`](docs/multi-agent-sync-runbook.md).

---

## Documentation

| Doc | Purpose |
|---|---|
| [`docs/codebase-map.md`](docs/codebase-map.md) | Task-oriented "if you need X, edit Y" |
| [`docs/PROTOCOL_BRIDGE.md`](docs/PROTOCOL_BRIDGE.md) | Tauri ↔ reflect-protocol envelope spec |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Top-level layout + state flow |
| [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md) | End-user manual |
| [`docs/CHANGELOG.md`](docs/CHANGELOG.md) | Release notes |
| [`docs/gui/`](docs/gui/) | Historical design docs (M1 snapshot) |
| [`docs/multi-agent-sync-runbook.md`](docs/multi-agent-sync-runbook.md) | Vendor sync checklist |
| [`docs/mobile-ios-tailscale-blueprint.md`](docs/mobile-ios-tailscale-blueprint.md) | iOS + Tailscale setup (planned) |
| [`docs/headless-daemon.md`](docs/headless-daemon.md) | Daemon lifecycle CLI (planned) |
| [`AGENTS.md`](AGENTS.md) | Agent contract for the repo |
| [`docs/index.html`](docs/index.html) | Rendered docs website (open in browser) |

---

## License

MIT — see [`LICENSE`](LICENSE).