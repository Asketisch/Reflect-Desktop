# ReflectDesktop — User Guide

> Reflect Agent GUI for macOS / Linux / Windows (Tauri 2 + React 19).

## 1. Install

### Pre-built (macOS arm64)

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

## 2. Launch

- Click `~/Applications/ReflectDesktop.app` on macOS
- Or: `reflect-desktop` from any terminal (PATH install)
- Or for development: `pnpm tauri dev` (HMR, DevTools)

## 3. What you get in M1.x

> ReflectDesktop follows a feature-sliced layout mirroring CodexMonitor's
> `src/features/` (24 slices: about / app / apps / collaboration / composer
> / debug / design-system / dictation / files / git / home / layout /
> messages / mobile / models / notifications / plan / prompts / settings /
> shared / skills / terminal / threads / update / workspaces).

In M1.x, the active slices are:

| Slice | State (M1.x) | Roadmap |
|---|---|---|
| **layout** | three-pane with resizable splitters | stable |
| **messages** | 7 row types + Markdown + prismjs syntax highlighting | + Streamdown typewriter in M2.x |
| **composer** | IME-safe textarea + slash popup (47 commands) + 9 Tier A toolbar buttons | + file-mention + image attach in M2.x |
| **modals** | shell + 4 stub variants (Approval / Question / AskUser / PlanReady) | active event-driven rendering in M2.x |
| **statusbar / settings** | top + bottom bar with model / theme / vim flags; Display / Editor / Provider settings | + Memory / Skills / Permissions sections in M2.x |
| **sessions** | time-bucketed list (Now / Today / Yesterday / This week / Older) | + fork + archive + LRU in M2.x |
| other 18 slices | empty README placeholder | B1–B10 wave-by-wave |

## 4. IPC contract

The GUI consumes Reflect Agent through the local in-process
`reflect-core::AgentThread` exposed by the `reflect_*` Tauri commands.
The wire protocol is `reflect-protocol::Event` / `Submission` (zerocopy
via Tauri's JSON serializer).

```text
React 19 + Vite
   |  (Tauri commands + events)
   v
src-tauri/        (Rust shim)
   |  (in-process Arc + mpsc)
   v
reflect-core::AgentThread (M1.x: MinimalAgent stub; M2.x: real)
```

When the agent ships additional hooks/tools/builtins, sync them once:

```bash
bash scripts/vendor-sync.sh /path/to/Reflect-Agent v1.0.0
```

## 5. Bundle sizes

| Variant | Size |
|---|---|
| `target/release/reflect-desktop` (Mach-O) | 7.5 MB |
| `target/release/bundle/macos/ReflectDesktop.app` (debug + bundle) | 7.3 MB |
| Cross-platform targets via `pnpm tauri build` | .dmg / .msi / .deb / .AppImage |

## 6. Switching from ReflectAgent's TUI

`reflect tui` (in `Reflect-Agent` repo) and `reflect-desktop` (this repo)
share the same `~/.reflect/sessions/` JSONL store, the same model
registry, and the same MCP servers. You can switch back and forth
without losing state.

## 7. Cross-platform status

| Platform | Status (M1.x) |
|---|---|
| **macOS arm64** | ✅ built + smoke tested |
| Linux x86_64 | ✅ built (untested smoke) |
| Windows x86_64 | ✅ built |
| Linux arm64 | scaffolded; follow-up PR |

## 8. Troubleshooting

- **`binary not found: .../target/release/reflect-desktop`** — run
  `pnpm tauri build --bundles app` first.

- **`Permission denied for ~/.reflect/sessions/`** — `chmod 700 ~/.reflect`.

- **App icon missing on Linux** — install `libgtk-3-dev` + `libwebkit2gtk-4.1-dev`
  or follow Tauri's Linux prerequisites.

## 9. Roadmap

See `docs/ARCHITECTURE.md` for the technical layout and `docs/PRODUCT_LINKAGES.md`
for the four product-level integrations still on the M2.x board:

1. native tray (Show/Hide / New conversation / Quit)
2. global hotkey (Cmd+. → reflect_interrupt)
3. macOS dock visibility + badge count
4. cross-platform packaging matrix (.dmg / .msi / .AppImage / .deb)
