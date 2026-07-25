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
- `/` slash popup driven by `src/features/composer/slashCommands.ts` (mirrors TUI slash commands).
- `@` file mention, drag/drop + paste images.
- Send: `Cmd+Enter` (macOS) / `Ctrl+Enter` (Linux/Windows).
- Interrupt: `Cmd+.` / `Ctrl+.` → `reflect_interrupt`.
- Model picker, effort toggle, permission mode cycle, approval rules.
- Composer is owned by `src/features/composer/`; `src/features/messages/Composer.tsx` is a thin re-export shim.

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
- Tauri command surface split by domain under `src-tauri/src/commands/<domain>.rs`, re-exported from a thin `src-tauri/src/commands/mod.rs`; command bodies wrap `reflect_protocol::Op` plus diagnostic / config / tool / session / file / git / shell / allowlist / workspace / skill / memory / hook / search commands.
- Single push event `reflect_event` (payload = `reflect_protocol::Event`).
- `tokio::sync::broadcast` for session fan-out to multiple subscribers.

### Settings

- Display (theme: light / dark / system, accent colors, transparency, wallpaper).
- Editor (vim / output-style toggles).
- Provider (default provider + API key placeholders).
- `ConfigForm` covers every section in `vendor/reflect-config/src/schema.rs`; advanced raw TOML editor remains the release valve.

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
bash scripts/build.sh        # auto-detects OS, outputs native bundles
bash scripts/install.sh      # macOS: installs .app + binary
```

`scripts/build.sh` detects the current OS and emits the right bundle type:

| OS       | Default bundles   | Output path                                   |
|----------|-------------------|-----------------------------------------------|
| macOS    | `app,dmg`         | `target/release/bundle/macos/`                |
| Linux    | `deb,appimage`    | `target/release/bundle/{deb,appimage}/`       |
| Windows  | `msi`             | `target/release/bundle/msi/`                  |

Useful flags:
```bash
bash scripts/build.sh --fast          # release-fast profile (no LTO, ~2x faster)
bash scripts/build.sh --universal     # macOS universal arm64+x86_64 binary
bash scripts/build.sh --bundles=app   # override bundle list
bash scripts/build.sh --dry-run       # print commands without executing
bash scripts/build.sh --help
```

Equivalent npm scripts: `pnpm build:native`, `pnpm build:universal`, `pnpm build:fast`.

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
├── src/            # React 19 + Vite + feature-sliced components
│   ├── features/   # per-slice folders (shell/messages/composer/...)
│   ├── stores/     # agent store (Zustand) split into agent/ submodule
│   ├── services/   # agent hook + agentEventBus (compat + ref-counted fan-out)
│   ├── utils/      # bridge.ts (low-level invoke/listen) + commands/<domain>.ts
│   │                # + tauri.ts / commands.ts compat barrels + i18n/<domain>/
│   ├── styles/     # tokens.css + base.css + typography.module.css
│   └── types/      # generated from reflect-protocol's schema dump
├── src-tauri/      # Tauri 2 Rust backend (binary name: reflect-desktop)
│   └── src/
│       ├── state.rs / events.rs / dock.rs / menu.rs / shortcut.rs
│       │         / tray.rs / mcp.rs        # top-level integration modules
│       ├── commands/mod.rs + commands/<domain>.rs + commands/error.rs
│       └── hook_store.rs / memory_store.rs / shell_sessions.rs
│                     / workspace_state.rs   # private state helpers
├── Cargo.toml      # workspace root for vendor + app-core + src-tauri
├── docs/           # CHANGELOG · PROTOCOL_BRIDGE · codebase-map · ARCHITECTURE
│                    #   + USER_GUIDE · runbooks · gui/ (design history)
└── scripts/        # install.sh · vendor-sync.sh · build.sh
```

---

## Tauri IPC Surface

The command surface is defined per-domain in `src-tauri/src/commands/<domain>.rs`, re-exported through the thin `src-tauri/src/commands/mod.rs` and registered in `src-tauri/src/lib.rs` via `tauri::generate_handler!`. The full set spans `reflect_*` command families covering `Op` submission, session I/O, config persistence, tool/skill/memory/hook listings, file + git + shell + workspace + allowlist + search, plus `ping` and `reflect_set_dock_badge`.

Frontend wrappers live under `src/utils/commands/<domain>.ts` (new code) with `src/utils/tauri.ts` and `src/utils/commands.ts` retained as compatibility barrels. The low-level `invoke` / `listen` primitives live in `src/utils/bridge.ts`.

Push events: single channel `reflect_event` carrying `reflect_protocol::Event` (snake_case JSON discriminators across the `EventMsg` variants).

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
