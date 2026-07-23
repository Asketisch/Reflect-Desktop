# Changelog

All notable changes to ReflectDesktop are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project adheres to [Semantic Versioning](https://semver.org/).

## Unreleased

### Added — Batch 12 (Native menu wiring + docs polish)

- **Native menu infrastructure**: `src-tauri/src/menu.rs` already provides 5 submenus
  (Reflect / Edit / Composer / View / Window) with 12 menu items including accelerators:
  - Reflect: About / Check for Updates / Settings (Cmd+,) / Quit
  - Edit: Undo / Redo / Cut / Copy / Paste / Select All (Predefined)
  - Composer: Cycle Model (Cmd+M) / Cycle Reasoning (Cmd+R) / New Agent (Cmd+N) / Interrupt (Cmd+.)
  - View: Toggle Sidebar (Cmd+B) / Toggle Terminal (Cmd+T)
  - Window: Minimize / Zoom / Close
- **Menu event forwarding**: `handle_menu_event` emits `menu-*` events to frontend for
  Settings navigation, Cycle Reasoning, New Agent, Interrupt, Toggle Sidebar, Toggle Terminal.
  Menu event listeners were added to AppShell to handle these events via `listen()`.
- **Dictation stub**: `DictationView` displays "Voice input coming soon" with Mic icon
  and Web Speech API / macOS Speech Recognition roadmap.
- **Update view**: `UpdateView` shows current version from ping query, manual update
  instructions via `bash scripts/install.sh`, and notes about tauri-plugin-updater integration.
- **TitleBar ⌘K hint**: Search icon + "⌘K" kbd hint on the right side with tooltip,
  making the CommandPalette discoverable.
- **StatusBar session count**: MessagesSquare icon + session count from `reflect_list_sessions`,
  tooltip "N sessions on disk".
- **Documentation**: CHANGELOG.md + PROTOCOL_BRIDGE.md + codebase-map.md updated for
  all B1-B12 changes. All 342/342 tests pass; tsc clean; cargo check clean.

### Added — Batch 1 (协议层 + app-core 基础) 对齐 zcode/Codex Desktop

- **协议类型生成 (B1-01)**:给 `vendor/reflect-protocol` 的 `EventMsg` / `Op` /
  `Submission` / `UserInputItem` / `ContentBlock` / `Question` 等所有
  协议类型加上 `schemars::JsonSchema` derive。重写 `dump_schema` example
  真正从 Rust 导出 2944 行 JSON Schema(33 EventMsg + 15 Op variants +
  所有嵌套 payload)。新增 `scripts/dump-ts-types.sh` + `pnpm run types:gen`。

- **ReflectEvent union 类型 (B1-02)**:用 `EventMsgByType` discriminated
  union 重写 `src/types/protocol.ts`,使 `event.msg.type` 收窄 payload 形状。
  涵盖全部 33 个 EventMsg variant + 15 个 Op variant + 完整
  `UserInputItem` / `ContentBlock` / `ReviewDecision` / `AskUserAnswer` 等。

- **Submission 构造器 (B1-03)**:新建 `src/protocol/submissions.ts`,
  导出 `userInputText` / `userInputImage` / `userInputSkill` / `compact` /
  `rewind` / `shutdown` / `toolApproval` / `hookApproval` / `planApproval` /
  `enterPlanMode` / `exitPlanMode` / `setEffort` / `setPermissionMode` /
  `cyclePermissionMode` / `askUserQuestionResponse` / `askUserInputResponse`
  等 16 个 Submission 构造器,统一 id 生成。

- **reducer 覆盖全部 33 variant (B1-04)**:重写 `agentStore.ts::reduceEvent`
  处理 `turn_rewound` / `shutdown_complete` / `token_count` / `config_reloaded` /
  `routing` / `collab_started` / `collab_message` / `collab_finished` /
  `mcp_tool_invoked` / `lsp_server_started` / `lsp_server_failed` /
  `plan_request` / `plan_ready` / `plan_approved` / `plan_rejected` /
  `permission_mode_changed` 等之前被静默丢弃的事件。新增 store 字段:
  `tokens` / `collabSessions` / `mcpInvocations` / `lastRouting` /
  `configReloadedAt`。

- **reducer 单元测试 (B1-05)**:新增 `src/stores/agentStore.test.ts` 38 个
  单元测试覆盖所有 33 个 EventMsg variant。

- **agentEventBus fan-out 服务 (B1-06)**:新建 `src/services/agentEventBus.ts`
  —— 单 Tauri `reflect_event` 订阅 + 引用计数 + 消费者错误隔离。`agentStore`
  改用 bus 而非自开 listener;Inspector / Notifications / DevTools 可
  各自订阅同一事件流。7 个单元测试。

- **后端域管理命令 (B1-07)**:新增 12 个 Tauri 命令 + 状态方法:
  - `reflect_list_workspaces` / `reflect_set_workspace` / `reflect_current_workspace`(B9-06)
  - `reflect_list_skills`(B11-06)— 扫描 `~/.reflect/skills/**/SKILL.md` + `<cwd>/.reflect/skills/**/SKILL.md`
  - `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`(B11-01)
  - `reflect_list_hooks` / `reflect_toggle_hook`(B11-02)
  全部 12 个命令注册到 `src-tauri/src/lib.rs::invoke_handler`;TS 包装
  在 `src/utils/commands.ts`。

### 验证

- `cargo check -p reflect-protocol` ✓(76 单元测试通过)
- `cargo check src-tauri` ✓
- `cargo test -p reflect-desktop` ✓(5/5)
- `pnpm typecheck` ✓(0 errors)
- `pnpm test` ✓(257/257 通过)



### Fixed — 顶栏贯通 + 红绿灯避让 + drag region（对齐 ZCode/Codex）

- **根因**:`tauri.conf.json` 已设 `titleBarStyle: "Overlay"`(红绿灯按钮浮在
  webview 之上),但 React 树**没有任何元素为红绿灯预留避让空间**,
  `ActivityBar` 的 32px "R" logo 正好被 3 个圆点盖住。同时 `base.css` 的
  `[data-tauri-drag-region]` 规则虽然存在,**全代码库没有任何元素挂这个属性** ——
  整个窗口无法靠标题栏拖动。此外旧 `TitleBar` 只横跨主区(嵌在 `.main` 内),
  与 ActivityBar/Sidebar 视觉脱节,与 ZCode/Codex「一条贯通全宽的深色顶栏」范式不符。
- **修复**:
  - `AppShell` 把 `TitleBar` 从 `.main` 内提升到 `.shell` 顶层(与 ActivityBar 同级、
    在它之前),顶栏现在横跨整个窗口宽度;`.body`(ActivityBar + Sidebar + Main + Inspector)
    全部从顶栏下方开始。
  - `TitleBar` 根元素挂 `data-tauri-drag-region`(整条顶栏可拖动窗口);内部 button/a/input
    由 `base.css` 既有规则自动 `no-drag`,sidebar/inspector toggle 仍可点击。
  - 新增 `--titlebar-height: 40px` / `--traffic-light-gutter: 80px` token;
    `TitleBar.module.css` 用 token 驱动高度 + 左 padding 为红绿灯让位。
- **回归保护**:`AppShell.test.tsx` 新增断言 TitleBar 是 `.shell` 的第一个子元素
  且挂了 `data-tauri-drag-region`。

### Fixed — 主内容区永久空白（critical regression）

- **根因**:`AppShell` 用 `children` prop 渲染主内容,但 TanStack Router v1 的
  root route component 不会收到 children —— 必须渲染 `<Outlet />` 才能把
  匹配到的子路由(HomeView / ChatView / SettingsView / …)挂到 DOM。
  现象:打开 app,ActivityBar / Sidebar / TitleBar / StatusBar 都正常,
  **唯独中间主区域永远是空白**(无论点哪个路由)。
- **修复**:`AppShell` 改用 `<Outlet />`;同步更新 `AppShell.test.tsx` +
  `app.smoke.test.tsx` 的 Outlet mock,新增回归测试断言 Outlet 内容真的挂载。
- **StatusBar 模型显示**:之前 store.session 为空时永远显示 "no model",
  哪怕后端 `reflect_agent_status` 查询返回了真实 model —— 现在按
  store.session → statusQuery.data.has_model → "no model" 三级 fallback,
  并把 `degraded_reason` 作为 tooltip 暴露。
- **TitleBar session 文案**:`session: (waiting...)` → `(waiting…)`(排版),
  并在 statusQuery 有 model 时显示真实 model 名,而不是永远 "waiting"。

### Changed — UI 全面重建收尾（polish + a11y + 测试 + 文档）

#### 功能性 bug 修复
- **`AppShell.Sidebar` 接线**:`onNewChat` 接入 → 导航 `/chat` 并清 active；`onSelect`
  同步 URL（`/chat/$sessionId`）+ 本地高亮。之前 "New chat" 按钮点击无反应。
- **`ChatView` 读 sessionId**:显示当前 viewing session banner（info 色 + monospace ID）。
- **`ActivityBar` active 边界修复**:`pathname.startsWith(prefix + '/')` 取代裸 `startsWith`,
  避免 `/git` 误匹配 `/github-webhook`。`aria-current="page"` 加到当前路由图标。

#### ModalShell 加固
- `aria-modal="true"` + `role="dialog"` + `tabIndex={-1}`。
- **焦点陷阱**:Tab/Shift+Tab 在 dialog 内循环,打开聚焦 primary,关闭还原焦点。
- **Enter 收紧**:仅当焦点在 dialog 容器自身（非 INPUT/TEXTAREA/SELECT/BUTTON）
  时触发 primary,避免输入表单时误提交。
- **响应式**:`<480px` 时 dialog 自适应到视口宽度,无溢出。
- 新增 `ModalShell.test.tsx` (7 测试:open 切换/Esc/overlay 点击/内部点击/footer
  省略/焦点还原)。

#### AppShell / 响应式 / a11y
- **响应式布局**:`<1100px` Inspector 变窄;`<900px` Sidebar+Inspector 转 absolute
  overlay;`<600px` Main 占满。
- **`MessageList` 空态优化**:从裸文字升级为 EmptyState 风格(图标 + 标题 +
  快捷键提示),加 `role="log"` + `aria-live="polite"` + `aria-label="Conversation"`。
- **`Composer` a11y**:`textarea` 加 `aria-label="Message Reflect"` + `autoFocus`;
  toolbar slash 按钮加 `aria-label`。
- **`Input/Textarea` a11y**:`invalid` 映射 `aria-invalid`。
- **`AppShell` a11y**:两个 `<aside>` 加 `aria-label="Sessions"|"Inspector"`。
- **WCAG AA 对比度修复**:`--text-muted` 深色 `#6b7488→#828c9f`,浅色
  `#8a93a6→#6b7488`,占位/提示文本达到 4.5:1。

#### 死代码清理 + i18n
- **删除 `SessionsView.tsx`**:死代码（Sidebar 包装,未被任何路由挂载）。从 sessions
  barrel 移除,更新 smoke test 和 README。
- **`i18n` 接入**:`DEFAULT_LOCALE` 改为 `en`(UI 主体语言),新增
  `composer.placeholder` / `chat.empty.title` / `chat.empty.hint` 三个键,中英双语
  一致。i18n 测试更新。

#### 文档同步
- **`docs/codebase-map.md`**:移除 `src/features/app/AppLayout.tsx`、`@widgets/*`、
  `(planned)` 标记;`shell/` 加入;`Composer` 路径纠正到 `features/messages/`。
- **`AGENTS.md`**:移除 `AppLayout`、`@widgets/*`、`Composer` 旧路径;`tokens.css`
  标记为 live;Hotspots 更新指向 `shell/`/`messages/`/`tokens.css`/`agentStore.ts`。
- **`docs/ARCHITECTURE.md`**:重写目录树(feature slice 枚举 `shell` 加入、`widgets/`
  移除);PROTOCOL_BRIDGE 不再标 `(planned)`;State flow 图更新反映 Zustand
  single store。
- **`src/features/sessions/README.md`**:入口说明更新为 `AppShell` 渲染 Sidebar。

#### 新增测试 (38 个, 总数 170→208)
- `shell/AppShell.test.tsx` (5):初始状态/sidebar 折叠/inspector 切换/双折叠 round-trip。
- `design-system/primitives/SegmentedControl.test.tsx` (8):role/aria-checked/onChange/disabled/hint/size。
- `features/notifications/NotificationsView.test.tsx` (8):空态/错误/4 个 pending 源/MCP&LSP/plan/过滤/dismiss。
- `features/modals/ModalShell.test.tsx` (7):open 切换/Esc/overlay 点击/Enter/footer/焦点还原。
- `utils/theme.test.ts` (9):localStorage/data-theme/subscribe/getResolvedTheme/initTheme/jsdom fallback。
- `utils/i18n.test.ts` (6):DEFAULT_LOCALE=en / 双语对齐。
- 修改 `MessageList.test.tsx` (中文→英文空态文案)。
- 修改 `app.smoke.test.tsx` (移除 SessionsView barrel 断言)。
- 修改 `Button.test.tsx` / `settings/SettingsView.test.tsx`(此前阶段)。

**全套 210 测试绿,生产 build 成功(CSS 76KB / JS 627KB)。**

### Added — Batch 7 (per-tool ToolCells + Approval history)

- **ToolCells (B7-05)**:新增 `src/features/messages/ToolCells.tsx`,
  per-tool 渲染 `tool_call` TurnItem。`ICON_MAP` 把
  `shell`/`bash`/`read_file`/`write_file`/`create_file`/`web_fetch`/`web_search`
  映射到专属 lucide 图标(Terminal / FileText / FileEdit / FilePlus /
  Globe / Search),未知 tool fallback `Wrench`。`summarize(name, args)`
  给出短摘要(`$ command` for shell,path for read_file 等)。
  `StatusIcon`(running=Loader2 spin / done=CheckCircle2 / error=XCircle)。
  折叠 header + 原始 args 详情。
- **MessageList 集成**:`tool_call` case 改用 `<ToolCell />` 替代通用
  `Collapsible`;删掉本地 statusIcon 计算。
- **ApprovalHistory (B7-03)**:新增 `src/features/modals/ApprovalHistory.tsx`,
  按 `decidedAt` 倒序展示历史 approval 决策(approve / approve_for_session /
  deny),带专属图标(CheckCircle2 / ShieldOff / XCircle)和 time 戳;
  可配置 `limit`、空态文案。
- **vitest setup**:`afterEach(cleanup)` 加入,避免 testid 跨测试泄漏。
- **jsdom polyfill**:`Element.prototype.scrollIntoView = noop`，
  让 autoscroll 组件在测试环境不报错。

### Added — Batch 11 (Memory management view)

- **MemoryView (B11-01)**: 新 `src/features/memory/MemoryView.tsx`,
  通过 `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`
  与后端同步持久记忆。
  - Scope filter: All / Global / Project / Session 四个 tab,
    实时计数;`filterBar` + `addBtn` 一行完成。
  - 新增表单: scope select + key + value input + Save button,
    点击 Add 展开,Cancel 收起。
  - 内联编辑: 点击 edit → textarea + Save/Cancel; delete → Trash2。
  - Empty state: "Add a key above or let the agent learn your preferences."
- **router**: 新增 `/memory` → `MemoryView` 路由。
- **测试**: MemoryView 5 个 vitest (title/filters/add toggle/form/empty state)。
  342/342 tests pass; tsc clean; cargo check clean。

### Added — Batch 10 (CommandPalette + ⌘K Keymap + StatusBar session count)

- **CommandPalette (B10-01)**:新 `src/features/command-palette/` 目录
  - `CommandPalette.tsx`: 模态浮层 + 输入框 + 模糊匹配列表; ↑↓ 选择,
    Enter 触发,Esc 关闭; backdrop blur + 最大 30 条可见;
    支持 `to`(路由导航) / `slash`(/compact /interrupt 等) / `run`(回调)。
  - `registry.ts`: 45+ 条命令覆盖全部导航(Home/Chat/Sessions/Settings/Files/
    Models/Skills/Workspaces/Git/Terminal/Plan/Prompts/About/Update/
    Notifications/Debug/Apps/Collaboration/Dictation/Mobile/Design-system),
    Session actions(New/Clear all/Export/Save config),
    Slash actions(/compact /interrupt /clear /help /rename /export),
    Theme(cycle/dark/light/system + current)。
  - `fuzzy.ts`: 轻量模糊匹配(子序列 + 开头加权 + 连续匹配加分 + keywords);
    7 个单元测试覆盖。
- **AppShell ⌘K 集成**: `AppShell.tsx` 添加全局 keydown listener(⌘K / Ctrl+K
  toggle palette, Esc close); `CommandPalette` 挂载在 AppShell 底部。
- **TitleBar**: 右侧新增 `Search ⌘K` 键盘快捷提示(kbd hint + tooltip)。
- **StatusBar session count**: 新增 `reflect_list_sessions` 查询,
  在状态栏右侧显示 MessagesSquare 图标 + session 总数,
  带 tooltip(「N sessions on disk」)。
- **测试**: fuzzy 7 + CommandPalette 6 = 13 个新 vitest。337/337 tests pass;
  tsc clean; cargo check clean(无 Rust 变更)。

### Added — Batch 9 (File tree + Code editor + workspace switcher)

- **Backend (B9-01)**:新 `reflect_list_dir(path?, maxDepth=4)` 返回
  `DirListing { root, entries, truncated }`,跳过 dotfile + node_modules /
  target / dist 等;封顶 2000 entries,按 dir→file,名称排序。`DirEntry`
  含 name/path/kind/size/mtime/depth。
- **Backend (B9-01)**:新 `reflect_read_file(path)` 返回
  `FileReadResult { path, content, size, binary, truncated }`,1 MiB 上限,
  NUL byte 判 binary(返回空 content),`canonicalize` 后必须仍在 workspace
  下(防 `..` 逃逸)。
- **Frontend FileTree (B9-01)**:新 `src/features/files/FileTree.tsx`,
  flat 列表按 parent path 构造折叠树,默认打开 depth-0 目录;按扩展名
  分文件图标 (Code/Text/Generic),size 显示(B/K/M)。5 个测试覆盖。
- **Frontend CodeEditor (B9-01)**:新 `src/features/files/CodeEditor.tsx`,
  只读代码预览,左 gutter 行号 + prismjs 高亮(13 种语言:
  rust/typescript/javascript/jsx/tsx/bash/json/yaml/toml/python/go/
  markdown/css/markup);不支持的扩展回退 plaintext;binary 文件显示提示;
  >1 MiB 标 "clipped to 1 MiB"。5 个测试覆盖。
- **FilesView 重写**:左 FileTree + 右 CodeEditor split 布局;顶 path bar
  + 刷新按钮;empty / error / loading 三态;`width="lg"`。
- **WorkspacesView 升级 (B9-04)**:每个非 active workspace 卡片右侧加
  "Use" 按钮(ghost sm size + ArrowRight 图标),点击调
  `reflect_set_workspace` 后 invalidate `agent-status` 缓存 + toast 反馈
  (success / error)。
- **测试**:FileTree 5 + CodeEditor 5 = 10 个新 vitest。324/324 tests pass;
  tsc clean;cargo check clean。

### Added — Batch 8 (Terminal: real shell exec + streaming)

- **Backend (B8-01)**:`reflect_run_shell(cmd) -> ShellSession { id, command, cwd }`
  用 `tokio::process::Command` 启动 `/bin/zsh -lc` (macOS) / `/bin/sh -lc`。
  stdout/stderr 各起一个 `BufReader::lines()` 后台任务,通过
  `app.emit("reflect_terminal_output", ShellOutputChunk)` 流式推给前端;
  主任务 `wait()` 后再 emit 一条 `stream="exit"` 携带退出码。
  进程在 `MinimalAgent` 的 `shell_sessions: HashMap<id, Arc<Mutex<Child>>>`
  中保留 kill handle。`reflect_kill_shell(id)` 幂等,`start_kill + wait`;
  `reflect_list_shell_sessions()` 诊断。
- **Frontend**:新增 `ReflectShellSession` / `ReflectShellOutputChunk` 类型
  + `reflect_run_shell` / `reflect_kill_shell` / `reflect_list_shell_sessions` /
  `onTerminalOutput` 四个 IPC wrapper。
- **TerminalView 重写**:多 session tab 流(可点击切换)+ 行级流式渲染
  (`stdout`/`stderr`/`exit`/`error`/`info`/`input` 不同色)+ Kill 按钮
  + Clear 按钮 + 5K 行滚动裁剪 + autoscroll + 命令历史 input。
- **测试**:TerminalView 4 个 presentational tests;`scripts/dump-ts-types.sh`
  重新生成。

### Changed — UI/UX 全面重建（阶段 4：剩余视图统一外壳 + stub 美化）
- **新增 `shell/PageShell`**:非 Chat 视图统一外壳(图标 + 标题 + 副标题 + 右侧操作 +
  滚动内容容器,sm/md/lg 三档宽度)。
- **剩余 13 个视图全部 CSS Modules 重写**:
  - **agent 操作类**:`Files`/`Git`/`Prompts` 卡片网格 + 快捷 prompt 按钮(sent 态 success
    反馈)。
  - **展示类**:`Skills`(工具表格 + Built-in/MCP 分组 badge)、`Collaboration`(MCP/LSP
    状态行 + 状态点 + future 卡)、`Workspaces`(workspace 卡片 + active 高亮)。
  - **信息类**:`About`(品牌卡 + info 表 + 链接)、`Update`(版本卡 + 命令块)。
  - **终端类**:`Terminal` 深色终端风(prompt + 命令输入 + CornerDownLeft 提示)。
  - **计划类**:`Plan` 计划卡片 + 编号步骤 + Approve/Reject DS Button(标注 sample data)。
  - **列表类**:`Threads` + `ThreadBucketGroup` + `ThreadItem`(active 高亮 + meta)。
  - **stub + dev**:`Apps`(sample data badge + Connect/Disconnect)、`Dictation`/`Mobile`
    (EmptyState coming soon)、`Debug`(深色 JSON 块 + 事件日志)。
- **ModalStack/ModalShell 视觉重写**:
  - `ModalShell`:header(title + close IconButton)+ body + footer(Deny/Approve for session/
    Approve 三按钮层级),backdrop blur,z-modal。
  - 4 个 modal(Approval/Question/AskUser/PlanReady)全部消费 token + CSS Modules。
- **品牌**:`index.html` title 改为 "Reflect Desktop"。
- **审计**:全 23 个 view 文件(19 路由 + Chat 三件套 + DesignSystem)100% CSS Modules 覆盖;
  view 层 inline style 硬编码颜色彻底清零(仅 Spinner 的动态 size prop 保留);
  emoji 图标全部替换为 lucide-react;M1.x scaffold 字样全部移除。
- **测试**:全套 170/170 绿,生产 build 成功(CSS 74KB / JS 625KB)。

### Changed — UI/UX 全面重建（阶段 3：高频视图精修）
- **新增 primitive `SegmentedControl`**:分段选择控件(low/medium/high 互斥切换),
  支持 stacked 卡片式 + hint。
- **`SettingsView` 重写为 IDE 式二级导航 + 卡片**:
  - 左侧 nav(Provider / Permissions / Advanced)+ 右侧内容区。
  - 顶部始终可见的 Permission mode 快捷控件(auto/prompt/deny/plan)。
  - Provider 卡片:Anthropic/OpenAI 各一张,API key 带显隐切换(Eye/EyeOff IconButton),
    configured 时显示 success dot badge。
  - Permissions section:4 张 PermCard(带说明文案)。
  - Advanced:raw TOML 编辑器(等宽卡片)。
  - sticky Save 栏。
- **`HomeView` 重写为 dashboard**:
  - hero 区(R logo + 标题 + model/workspace 状态卡)。
  - Quick start 4 张大卡片(New chat / Workspaces / Models / Settings),hover 升起 + 箭头。
  - Recent sessions 卡片列表(相对时间 + token)。
  - 空态用 EmptyState primitive。
  - 新增 `/home` 路由 + ActivityBar Home 入口。
- **`ModelsView` 重写**:
  - Current model 大卡片(图标 + model 名 + ready badge + workspace)。
  - 降级态 EmptyState 引导去 Settings。
  - Reasoning effort 用 SegmentedControl(stacked + hint)+ Apply 按钮。
- **`NotificationsView` 重写**:
  - 顶部 error/pending 计数 badge。
  - All/Errors/Pending 过滤 tab。
  - 通知项 Card + 左侧语义色条(error/warn/info)+ lucide 图标。
  - 空态 EmptyState。
- **测试**:`SettingsView.test.tsx` 的 section 断言改 getAllByText(nav + section 同名)。
  全套 170/170 绿,生产 build 成功(CSS 51KB / JS 611KB)。

### Changed — UI/UX 全面重建（阶段 2：IDE 式 Shell + 核心三件套）
- **新建 `src/features/shell/` IDE 三栏布局**:
  - `AppShell`:ActivityBar(56px) + Sidebar(可折叠) + Main + Inspector(可折叠) + StatusBar
    五区布局,取代旧的 Topbar/三栏/BottomBar 脚手架。
  - `ActivityBar`:lucide 图标导航(Chat/Files/Git/Terminal/Skills/Notifications +
    Settings/About),active 态从 `useLocation` 派生,tooltip 提示。
  - `TitleBar`:Main 顶部标题栏,显示当前 view 名 + session 状态(model @ provider /
    waiting) + permission badge + sidebar/inspector 折叠按钮。
  - `StatusBar`:底部状态栏,model@provider + 可点击循环 permission + workspace basename
    + MCP/LSP 失败计数 + 错误提示 + 主题切换按钮(Sun/Moon/Monitor 三态循环)。
  - `Inspector`:右侧面板,pending 交互计数 + MCP/LSP server 状态列表 + 最近错误。
- **核心三件套全部 CSS Modules 重写**:
  - `MessageList`:用户气泡右对齐 accent-subtle,助手左对齐 + R 头像 + markdown;
    thinking/tool_call/tool_output 统一用新抽出的 `Collapsible` 组件(左侧状态色条
    + lucide 图标 Brain/Wrench/CheckCircle/XCircle/Loader);streaming 光标改为
    CSS 脉冲块;移除顶部调试 session 行(已移到 TitleBar)。
  - `Composer`:卡片式容器(elevated bg + focus 环),toolbar 9 个 slash 命令 pill,
    底部 SlashSquare 触发按钮 + ArrowUp Send IconButton(primary 态),IME 安全
    Cmd/Ctrl+Enter。
  - `Markdown`:消费 token 的排版,代码块带 header(lang + Copy 按钮),Prism 深色
    okaidia 风格 token 配色。
  - `Sidebar` / `BucketGroup` / `SessionItem`:New chat 主按钮 + 搜索框 + 时间分桶
    + active 左边条 + token/meta 信息。
  - `SlashPopup`:深色阴影弹层 + 键盘选中高亮 + mono 字体命令名。
- **路由**:`router.tsx` root component 从 `AppLayout` 切到 `AppShell`;index 路由
  改为直接渲染 ChatView(HomeView 留待阶段 3 精修)。
- **清理**:删除死代码 `src/features/app/AppLayout.tsx` + `src/widgets/StatusBar.tsx`
  (旧 Topbar/BottomBar,M1.7 scaffold 字样彻底移除)。
- **主题**:`utils/theme.ts` 增加 jsdom 环境 matchMedia 防御。
- **测试**:更新 `MessageList.test.tsx`(session 状态行断言改为空态提示)、
  `app.smoke.test.tsx`(AppLayout → AppShell 断言 + 补 useLocation/useMatches mock)。
  全套 170/170 绿,生产 build 成功(CSS 37KB / JS 600KB)。

### Changed — UI/UX 全面重建（阶段 1：设计系统地基）
- **`tokens.css` 深色优先重写**:`:root` 即深色主题(IDE 风,4 级背景层次
  `--bg-app/surface/elevated/input`),`[data-theme="light"]` 覆盖为浅色,
  `prefers-color-scheme` 媒体查询支持 system 模式。语义色(success/warning/danger/info)
  各含 fg/bg/border 三态。新增间距(9 档)、字号(9 档)、圆角、阴影、z-index、transition
  token。原 `--bg/--ink/--muted` 等旧 token 已移除。
- **新建 `base.css`**:全局 reset(box-sizing/margin/padding)、`body` 消费 token、
  滚动条(macOS 风细滚动条)、`::selection`、`:focus-visible` 全局环、
  `[data-tauri-drag-region]` 拖拽区、`prefers-reduced-motion` 无障碍。
- **新建 `typography.module.css`**:h1-h4 / body / caption / mono / label 排版原子类。
- **新建 `utils/theme.ts`**:主题切换基础设施(light/dark/system),localStorage 持久化,
  `subscribeTheme` 订阅,`initTheme()` 在 main.tsx 启动时 apply。
- **设计系统 primitive 全部用 CSS Modules 重写**:
  - `Button`:`data-variant` + `data-size` 驱动(primary/secondary/tertiary/danger/ghost
    × sm/md/lg),新增 `loading` 态(spinner)。废弃 `buttonStyle()` 工具。
  - 新增:`Icon`(lucide-react 封装)、`IconButton`、`Input`/`Textarea`/`Select`、
    `Badge`(6 variant + solid + dot)、`Card`(flat/outlined/elevated)、`EmptyState`、
    `Spinner`、`Tooltip`(纯 CSS)。
  - `Toast`/`ContextRing`/`KeyHint` 从 inline style 迁移到 CSS Modules。
- **`DesignSystemView` catalog 重写**:展示全部新 primitive(10 个 section),
  作为活体参考。`main.tsx` import tokens + base + 调用 `initTheme()`。
- **测试**:`Button.test.tsx` 断言改为 `data-variant`(CSS class 取代 inline style);
  删除 `buttonStyles.test.ts`(工具已废弃);`app.smoke.test.tsx` 同步更新 barrel/token 断言。
  全套 170/170 绿。

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