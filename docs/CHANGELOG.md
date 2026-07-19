# Changelog

All notable changes to ReflectDesktop are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project adheres to [Semantic Versioning](https://semver.org/).

## Unreleased

### Added
- **跨平台打包脚本 `scripts/build.sh`**:自动检测 OS(macOS/Linux/Windows),
  输出对应原生包(macOS→app+dmg / Linux→deb+appimage / Windows→msi)。
  支持 `--universal`(macOS arm64+x86_64 合一)、`--fast`(release-fast profile)、
  `--bundles=LIST` 覆盖、`--dry-run` 预览。配套 npm scripts:`build:native` /
  `build:universal` / `build:fast`。
- **CI workflow `.github/workflows/release.yml`(备用)**:`push tag v*` 触发,
  三平台并行构建(macos-14/macos-13/ubuntu-22.04/windows-latest),自动上传
  artifacts 到 workflow run + tag release。日常 push 不触发。本地构建不依赖它。
- **`tauri.conf.json` 平台专属字段补全**:修正 `homepage` 指向 ReflectDesktop 仓库;
  新增 `copyright`;`macOS.minimumSystemVersion = "11.0"`;`windows.webviewInstallMode =
  downloadBootstrapper`(用户无 WebView2 时自动下载);`linux.deb.depends` 对齐
  README 文档化的 webkit2gtk-4.1 / libayatana-appindicator3 / librsvg2 依赖。

### Added
- **真实 agent 集成(阶段 1)**:`MinimalAgent` 不再用空 `ModelRegistry` +
  `EchoTool` stub。现在经 `reflect_config::load_default()` 读 `~/.reflect/config.toml`
  + 环境变量(`OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `OLLAMA_HOST` /
  `REFLECT_PROVIDER` / `REFLECT_MODEL`),`cfg.to_registry()` 构建真实 provider pool,
  注册 16 个 reflect-tools 内置工具(Bash/Read/Write/Edit/Delete/Grep/Glob/
  NotebookEdit/EnterPlanMode/ExitPlanMode/EnterWorktree/ExitWorktree/WebFetch/
  WebSearch/ToolSearch/Echo)。
- **降级策略**:开发环境无 API key 时 fallback 到空 registry + EchoTool + warn,
  `pnpm tauri dev` 永远能起;新增 `reflect_agent_status` 命令暴露
  `{ready, has_model, model, workspace, degraded_reason}` 供前端显示状态徽标。
- **12 个 Op 命令实装(阶段 2)**:`reflect_compact` / `reflect_rewind` /
  `reflect_shutdown` / `reflect_tool_approval` / `reflect_hook_approval` /
  `reflect_plan_approval` / `reflect_enter_plan_mode` / `reflect_exit_plan_mode` /
  `reflect_set_effort` / `reflect_set_permission_mode` / `reflect_cycle_permission_mode` /
  `reflect_ask_user_question_response` / `reflect_ask_user_input_response` 不再是
  空 `Ok(())`,改为构造对应 `Op` 经 `MinimalAgent::submit_op` 真正驱动 AgentThread。
- **新增命令**:`reflect_agent_status` / `reflect_get_config` / `reflect_save_config` /
  `reflect_list_tools`(诊断 + 配置持久化 + 工具列表)。
- **Zustand agent store(阶段 3a)**:`src/stores/agentStore.ts` 单 store + 单次事件订阅,
  修复 M1.x 各组件持独立 turns 副本的结构 bug;`Turn` 模型从 `{user,reply,done}`
  升级为 `{id,items,status}`,`TurnItem` 覆盖完整 33 种 `EventMsg`(user_text /
  assistant_text / thinking / tool_call / tool_output / error / compacted)。
- **富文本 Chat 渲染(阶段 3b)**:`MessageList` 按 `turn.items[]` 渲染多行;工具调用
  可折叠(参数 + 输出 + 状态徽标)、thinking 可折叠、错误红框、compacted 提示。
- **真 modals(阶段 3c)**:`ModalStack` 从 store 的 pending 队列渲染 ApprovalModal /
  QuestionModal / AskUserModal / PlanReadyModal,提交调 store action。
- **MCP / LSP 运行时接入(阶段 3d)**:vendor `reflect-mcp` + `reflect-lsp` crate;
  `src-tauri/src/mcp.rs` 经 `McpConnectionManager` / `LspConnectionManager` 启动
  `[mcp_servers.*]` / `[lsp_servers.*]` 配置的 server,MCP 工具 `register_if_absent`
  到 ToolRegistry,LSP 单例 `LspTool`;lifecycle event 经 session broadcast 推前端。
- **Settings 真持久化(阶段 4)**:SettingsView 经 `reflect_get_config` /
  `reflect_save_config` 读写 `~/.reflect/config.toml`;结构化 API key / provider /
  model 编辑 + 高级 raw TOML 编辑器(MCP/LSP/sanitize);ModelsView 从真实
  `agent_status` 读当前 model。
- **STUB view 真实化(阶段 5)**:Skills/Notifications/Collaboration 接 store 真实数据;
  Workspaces 从 sessions 聚合;Prompts/Files/Git/Terminal 改为"发送到对话"快捷面板;
  About/Update 用真实 `ping` + `agent_status`。
- **Docs**: `docs/codebase-map.md` — task-oriented navigation.
- **Docs**: `docs/PROTOCOL_BRIDGE.md` — Tauri ↔ reflect-protocol envelope spec.

### Changed
- `ReviewDecision` TS 类型对齐 Rust `#[serde(rename_all = "snake_case")]`:
  从 `'approve'|'deny'|'abort'` 改为 `'approve'|'approve_for_session'|{deny:{reason}}`。
- `reflect_*` Op 命令返回类型从 `void` 改为 `string`(submission id,供 pairing)。
- `services/agent.ts` 改为 `stores/agentStore.ts` 的兼容 re-export 层。

### Removed
- `src/App.tsx`(死代码入口,真根是 `main.tsx` → `router.tsx`)。
- `commands/mod.rs::all_commands()`(dead code)。
- 所有 view 的硬编码 `STUB_*` 数组 + phantom `invoke()`(Files/Git/Terminal/About/Update
  曾 invoke 不存在的命令,违反 AGENTS.md 规则 5)。

### Fixed
- 修复 `useAgent()` 各组件持独立 turns 副本的结构 bug(改用 Zustand 单 store)。
- 修复 `handle_event` 只处理 4/33 事件类型、丢弃工具调用/思考/审批/错误的问题。
- 修复 Files/Git/Terminal/About/Update 调用不存在的后端命令的问题。

### Security
- 工具输出经 `Sanitizer::with_defaults()`(10 个默认密钥脱敏 pattern)。

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