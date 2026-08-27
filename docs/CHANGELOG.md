# 更新日志

ReflectDesktop 的所有重要变更均记录于此。格式遵循 [Keep a Changelog](https://keepachangelog.com/)，版本号遵守[语义化版本](https://semver.org/)。

## 未发布

### 功能 — GUI 会话持久化：M4 recorder 接线 + 跨轮记忆修复（v1.x）

- **协议/引擎层**（reflect-agent submodule）：
  - `SessionConfiguredEvent.session_id` 不再随机 —— `submission_loop` 在
    `SessionConfiguredEvent::new` 后覆盖为线程预分配 id（serve.rs 同），
    与 rollout 文件名 / `SessionMeta.session_id` / 前端路由 id 一致。
  - 新模块 `reflect_core::resume::records_to_preload(&[RolloutRecord]) ->
    Vec<ChatMessage>`：rollout 记录 → LLM 历史映射（legacy String / 图文
    blocks / 工具对顺序 / compaction 摘要前置 System），`bootstrap_resume`
    与 GUI 回填共用单一实现。
  - `submission_loop` **每轮历史回填**：有 recorder 的线程在每 UserInput
    turn 前从 `recorder.replay` 重建会话历史（修复跨轮失忆 —— 旧引擎
    `AgentState::default()` 每轮新建、`preload_messages` 一次性 take，
    模型每轮只见新输入）。无 recorder 线程保留 preload-once 语义。
  - `CronScheduler::rebind_sender(Option<Sender<Submission>>)`：换绑
    投递通道（纯字段赋值；调用方须 stop + 重启 driver，job uuid 经共享
    Arc 保留）。
  - `ToolRegistry::external_tool_names()` + `pre_loop` 外部工具可见性补丁：
    `effective_tools` 增并非 `Builtin` 源工具名 —— **M4 开启后 MCP/LSP/
    Plugin 工具不再被隐藏**（此前过滤为 always_on ∪ 已激活 skill，CLI 同受益）。
- **Tauri 适配层**：
  - 新命令 `reflect_bind_session({ id })`：replay 该 id 的 JSONL →
    `records_to_preload` → `state::rebind::rebind_session`（换绑线程：
    `construct_thread` 带 `with_session_id` + 完整 M4（`build_default_m4`，
    recorder 覆写为同 id 的 `JsonlRolloutWriter`），旧 turn `cancel_token`
    有界排空，cron driver 换 sender 重启；同 id 幂等短路；未知 id 空历史
    绑定不报错）。
  - 新模块 `state/thread_factory`：registry 构造 + `construct_thread(sid,
    preload)` 统一 install / rebind 两路径；M4 全量启用（skills / memory /
    compaction / notes），recorder base 统一到
    `reflect_rollout::path::default_base()`。
  - **cron 修复**：install 时 `let _driver_handle = driver.start(30)` 立即
    Drop 句柄 → driver 任务被 abort → **cron 任务从未真正触发**。句柄现
    存 `inner.cron_driver`，rebind 时 stop 旧 driver 后重启。
  - **rebind 写锁自死锁修复**：`rebind_session` step 5 的
    `if let ... = cron_scheduler.write().clone()` scrutinee 临时值（写
    guard）活到 if-let 块结束，块内再对同一把锁 `write()` 因 parking_lot
    非重入自死锁 → 每次 `reflect_bind_session` 永久挂起 IPC 线程。改为
    读锁 clone 释放 guard 后再取写锁写回。
  - **cron 换向提前**：旧 driver 的 stop 从新线程构造之后提前到旧线程
    cancel 之后立即执行 —— `CronScheduler::start` 按值捕获旧
    `sub_tx`，窗口内旧 driver 仍会把到期 job 投进已 cancel 的旧线程
    （静默丢触发）；新 driver 首个 tick 立即扫描补发，不丢触发。
  - `sessions_base()` 对齐 `default_base()`，杜绝列表/回放路径分叉。
- **前端**：`reflect_bind_session(id)` IPC 包装；`ChatView` 加载序列改为
  **bind → replay → hydrate**（已水合同 id 也先 bind 再早退，后端幂等；
  retry 同构）—— 切换会话后续写该会话文件且 LLM 恢复上下文。
- **行为变化**（需知）：
  - GUI 会话开始落盘 `~/.reflect/sessions/YYYY/MM/DD/<id>.jsonl`（首行
    `SessionMeta.workspace` = 归属工作区）。
  - 多轮记忆恢复（此前每轮失忆）；系统 prompt / 工具表与 CLI 同款
    （M4 全量），prompt 变长由 compactor 收敛。
  - 配置的 cron 任务开始真正按周期触发。
- **测试**：submodule 侧 `resume` 映射 6 用例 + `pre_loop` M4/回填/rewind
  3 用例 + `session_configured_reports_preallocated_session_id` +
  cron rebind 用例；GUI 侧 rebind 4 用例 + install registry/cron 句柄用例；
  前端 ChatView bind→replay 顺序 / 切会话重绑 2 用例 + fakeBackend /
  mock / contract 同步。
- **文档**：`docs/PROTOCOL_BRIDGE.md` 补 `reflect_bind_session` 行 +
  `SessionConfigured.session_id` 语义。

### 功能 — 工作区 ↔ 会话归属 + Composer `@` 文件弹层（v1.x）

- **协议层**（reflect-agent submodule，serde 全兼容 —— 新字段均带
  `#[serde(default, skip_serializing_if = "Option::is_none")]`）：
  - `UserInputItem` 新增 `File { path, range? }` 变体（`FileRange { start_line, end_line }`）；
    core 侧 `user_input_items_to_messages` 展开为 `@<path>`（含行区间时
    `@<path>:L<n>-L<m>`）文本块，真实读取由 LLM `/read` 工具按需触发。
  - `Submission.workspace: Option<String>` —— 会话归属随首条 UserInput 传递，
    后端写入 `RolloutRecord::SessionMeta.workspace`（回退 `cfg.current_workspace()`）。
  - `SessionInfo.workspace: Option<String>` —— 旧 JSONL 反序列化为 `None`，
    前端按"未归属"展示。
  - `rollout_index::list_sessions_in_workspace(base, ws)` —— 精确匹配过滤；
    旧会话（`workspace = None`）在过滤视图不出现（归属创建时确定，避免误归类）。
    JSONL 路径布局不变（`<base>/YYYY/MM/DD/<id>.jsonl`）。
- **Tauri 适配层**：新命令 `reflect_create_session -> string`（无参纯 ID
  分配，New Chat 先拿路由 id，workspace 归属经 `Submission.workspace`
  注入）；`reflect_list_sessions` 增加可选 `workspace`
  过滤参数（老 IPC 传 `null` 兼容全量列表）。
- **前端 — 会话归属**：新 hook `useCurrentWorkspace`（30s staleTime）；
  `AppShell.handleNewChat` 改为 `reflect_create_session()` →
  `setActiveId` → 导航；`useSessions` 支持 `workspacePath` 过滤（queryKey 含
  workspace 维度）；`reflect_set_workspace` 成功后失效 current-workspace +
  sessions 缓存；`SessionItem` 副标题显示归属项目名（basename，hover 全路径），
  未归属显示"未归属"。
- **前端 — `@` 文件弹层**：`useComposerInput` 增 `MENTION_QUERY` 检测
  （`(^|\s)@([^@\s]*)$`）+ Escape 关闭；`MentionPicker` 切到文件源
  （`reflect_list_dir(null, 2)` 浅层递归，跳过 node_modules/target/.git 等，
  复用 command-palette fuzzy，slice 50）；`useAttachments.addFileMention` +
  `toUserInputItems` file 映射；工具栏 `@` 按钮改为追加 `@` 并直接开弹层
  （显式 `setMentionVisible(true)` —— setState 不触发 onChange）；
  `useComposerSubmission` 透传 `workspace` 进 Submission 信封。
- **测试**：前端 7 组新增/扩展（@ 触发检测 / MentionPicker / addFileMention
  线格式 / useSessions workspace 过滤 / Composer @ 弹层集成 / AppShell New
  Chat / useCurrentWorkspace）；Rust `list_sessions_in_workspace` 过滤断言。
- **文档**：`docs/PROTOCOL_BRIDGE.md` 补 `File` / `FileRange` /
  `Submission.workspace` / `SessionInfo.workspace` schema 与
  `reflect_create_session` 命令行。

### 修复 — 窗口拖拽 / 项目目录 / 会话归档（GUI 三项可用性补齐）

- **窗口拖拽**：根因是 Tauri 2 的 `core:window:default` 权限集不包含
  `core:window:allow-start-dragging`，`data-tauri-drag-region` 触发的
  `plugin:window|start_dragging` IPC 被 ACL 静默拒绝，标题栏拖不动。
  - `src-tauri/capabilities/main.json` 新增 `core:window:allow-start-dragging`。
  - `src/features/shell/TitleBar.tsx` 改用 `data-tauri-drag-region="deep"`
    （Tauri drag.js 在子树内命中非交互元素即触发，整条顶栏可拖）；
    移除冗余的手写 `onMouseDown` preventDefault（drag.js 已自带）。
  - `src/styles/base.css` 与 `TitleBar.module.css` 移除无效的
    `-webkit-app-region: drag`（WKWebView 不识别此 CSS 属性，Tauri 拖拽是
    属性 + JS 命中 + ACL 三件套）。
- **项目目录**：WorkspacesView 接入真实历史列表 + 原生目录选择。
  - 后端 `commands/workspaces.rs`：
    - `reflect_set_workspace` 切换即把当前项 upsert 进
      `~/.reflect/workspaces.json`（去重 + last_used 刷新 + 截断到 20 条）。
    - `reflect_pick_workspace_folder`：原生目录选择对话框（Rust 侧
      调 `tauri-plugin-dialog` 的 `DialogExt::pick_folder`，经 oneshot
      折回 async 返回；不走 JS IPC，无需 dialog capability）。
    - `reflect_reveal_path`：在系统文件管理器中定位路径（macOS `open -R` /
      Windows `explorer /select,` / Linux `xdg-open`）。
  - 依赖：`src-tauri/Cargo.toml` 新增 `tauri-plugin-dialog = "2"`，
    `lib.rs` 注册 `tauri_plugin_dialog::init()`。
  - 前端 `src/features/workspaces/WorkspacesView.tsx` 重写：当前 workspace
    卡 + 「打开项目目录…」主按钮 + 「在文件管理器中显示」次按钮；最近
    项目区按 `last_used` 倒序，每条支持 reveal / use。
  - IPC 包装 `src/utils/commands/workspaces.ts` 补三个新 wrapper；
    `commands/mod.rs` 注册三个新命令 + `lib.rs::invoke_handler!` 同步。
- **会话归档 / 恢复 / 删除**：共享 `SessionItemMenu` 让侧边栏与
  ThreadsView 都能执行 rename / export / archive / delete。
  - 后端 `commands/sessions.rs`：新增 `reflect_archive_session` /
    `reflect_unarchive_session` / `reflect_list_archived_sessions`。
    归档把 session 的所有文件（含轮转副本、文件名错位副本）+ 自定义名
    搬到 `~/.reflect/sessions-archive`，相对路径不变（unarchive 沿同一
    相对路径搬回）。归档目录放在 sessions 树之外，避免被
    `rollout_index::list_sessions` 全树扫描命中。
  - 前端：抽出 `src/features/sessions/components/SessionItemMenu.tsx`
    （合并 `threads/components/ThreadItemMenu.tsx`），SessionItem 行尾
    kebab（⋯）悬停出现；`BucketGroup` / `Sidebar` / `ThreadsView` 透传
    菜单 handler；归档当前打开会话后自动回 `/chat`。
  - ThreadsView 新增「Archived」区（带 restore / 彻底删除）。
  - IPC 包装 `src/utils/commands/sessions.ts` 补三个 wrapper；
    `useSessions` hook 新增 `archive` / `unarchive` / `archived` 字段。
- **Capabilities / Tests / Docs**：
  - `tests/helpers/fakeBackend.ts` 同步四个新命令 + workspaces 线格式
    改为后端 `WorkspaceInfo`（unix 秒 + label）。
  - `tests/ipc.command-contract.test.ts` 加 archive round-trip + folder
    picker / reveal 路径断言。
  - `tests/sessions.lifecycle.test.tsx` 加侧边栏 kebab 归档 / 删除 /
    自动回 `/chat`、ThreadsView restore 归档全流程用例。
  - `tests/views.route-matrix.test.tsx` 改为校验
    `reflect_list_workspaces` + `reflect_agent_status`。
  - 新增 `src/features/sessions/components/SessionItemMenu.test.tsx`（6 用例）。
  - 后端 `commands/{workspaces,sessions}.rs` 加 7 个 Rust 单元测试（去重 /
    持久化 / 容量上限 / archive+restore 整树搬移），`cargo test --lib
    commands::` 48 个测试全绿。
  - `docs/PROTOCOL_BRIDGE.md` 新增 6 个命令条目 + 归档树 / workspaces.json
    持久化约定。

### 修复 — v1.x diff 审查修复（前端）

- **MentionPicker 路径契约**：后端 `DirEntry.path` 是绝对路径，弹层原样
  透传 → 违反 `UserInputItem.File.path` 工作区相对路径契约。现按
  `listing.root` 剥离前缀（hint 展示同走相对路径）；`onPickFile` 契约
  回归用例（绝对路径 mock → 相对路径断言）。
- **MentionPicker i18n**：`Files` / `Loading files…` / `No matching files.`
  硬编码英文改 `composer.mentionPicker.*` 键；补漏的
  `composer.toolbar.mentionFile` 键（工具栏 `@` 按钮此前取不到文案）；
  清理死键 `session.workspaceBadge` / `composer.mentionPicker.placeholder`。
- **ChatView 加载 effect**：unmount / 换 session 时递增 `requestIdRef`
  使在途 replay 作废（防旧 replay 水合进全局 store 污染新会话）；
  已水合判断改读 `useAgentStore.getState()` 实时值并把
  `loadedSessionId` 移出 deps —— 水合成功后不再二次重跑
  （多余 bind + loading banner 闪断 + 双份 JSONL 读）。
- **New Chat 失败兜底**：`handleNewChat` 补 `.catch` → error toast
  （新键 `toast.newSessionFailed`）。
- **SessionItemMenu 主题**：`--rd-*` 令牌全仓不存在，暗色 hex fallback
  在亮色主题下渲染成深色面板。改 `tokens.css` 既有令牌
  （`--bg-elevated` / `--border-default` / `--text-primary` / `--accent` …）。
- **`/goal` 斜杠命令**：缺参 reject 文案混排中英，改与 engine 其余
  消息一致的英文（engine 保持纯函数，文案走 caller toast）。
- **`Submission.workspace` 线格式测试**：新增 `stores/agent/store.test.ts`
  —— 非空注入顶层字段、null/undefined 省略字段（后端回退
  `cfg.current_workspace()` 的前提）。

### 新增 — goal 模式 GUI 触发面（Op 直通 + `/goal` 斜杠命令）

- **背景**：上游 `reflect-protocol` v1.2 P1 已有 `Op::EnterGoalMode { goal,
  verify_command?, token_budget? }` / `Op::ExitGoalMode`（core 侧
  `GoalController` 每轮 turn 结束自校验、未完成自动续作），但 Desktop GUI
  此前无触发面（i18n 标注"后续阶段"）。
- **后端**：`src-tauri/src/commands/agent.rs` 新增 `reflect_enter_goal_mode` /
  `reflect_exit_goal_mode` 两个薄 Op 适配（与其他 14 个 Op 命令同模式，经
  `MinimalAgent::submit_op` 投递），`lib.rs` 注册表同步。
- **前端**：新域文件 `src/utils/commands/goal.ts`（barrel 导出）；Composer
  新增 `/goal <description>` 斜杠命令（`/goal clear` 退出，与 TUI 约定一致），
  无参数时 reject 并给出引导。
- 文档：`docs/PROTOCOL_BRIDGE.md` §2 注记 + §6 Op 联合补两个变体。
- 测试：`tests/flows.chat-plan-goal-edit.test.tsx` 覆盖 goal 多轮自动续作
  （零用户提交下多 turn 累积/状态流转/退出）；契约测试补 goal round-trip
  （可选字段缺省/完整两形态 + 状态清除）。

### 新增 — `tests/` 应用级全量测试套件（168 测试，全绿）

- **基础设施**：`tests/helpers/fakeBackend.ts` 实现 `lib.rs` 注册表中的
  全部 `reflect_*` IPC 命令（带状态、失败注入、调用日志、线格式参数断言），
  内置 ScriptedAgent 事件编排（`emit` / 审批·提问·计划 `gate` 挂起，
  复刻真实 agent 审批停顿与提交抢占）；`tests/helpers/appHarness.tsx`
  挂载完整真实应用（真实 reducer / React Query / 路由 / 事件总线订阅）。
- **覆盖**：IPC 契约与四方对齐（后端函数 ↔ 前端包装 ↔ handler 注册 ↔
  模拟实现 + `PROTOCOL_BRIDGE.md` 覆盖率下限）、全部 `EventMsg` 变体
  协议矩阵、聊天端到端主流程（含审批门）、**chat → plan → edit 全链路**
  （plan 草稿多次更新 → plan_ready → 三选择决策 → 编辑工具逐审批 →
  完成，含 IPC 时序断言）、**goal 模式多轮自动续作**、审批/提问/计划
  四模态全路径、会话生命周期（分桶/重放/切换）、Composer 斜杠命令/
  历史/附件、设置页配置读写、34 条路由视图矩阵、错误与韧性（流错误/
  配额/后端故障/中断）。
- 运行：`pnpm vitest run tests/`；全量 `pnpm test` 现共 77 文件 732 测试。
- 详见 `tests/README.md`。

### 修复 — reflect-sandbox seatbelt 临时 profile 并发写竞争（上游 submodule）

- **现象**：`cargo test --workspace` 多线程下 `reflect-sandbox` 的
  `rm_rf_root_is_blocked_in_sandbox` 必挂（stdout 为空），单线程 / 单用例
  通过。**这不是测试问题,是 `seatbelt_argv` 的真实缺陷**:临时 profile
  固定写 `reflect-sandbox-<pid>.sb`,同一进程内并发调用(桌面应用并行
  工具执行、并行测试)在 `fs::write` 截断重写窗口内互相踩踏 ——
  `sandbox-exec` 读到半截 profile 编译失败、子进程不启动(fail-closed);
  或读到他人完整 profile,workspace 写白名单错乱。
- **修复**(Reflect-Agent 仓库,submodule 指针同步升级):文件名追加原子
  自增序号(`reflect-sandbox-<pid>-<seq>.sb`),每次调用独立文件;新增
  回归测试 `seatbelt_argv_temp_profiles_are_unique_per_call`。验证:
  `--lib` 多线程 5 连跑 15/15 全绿。
- 顺带:全量测试亦确认根 workspace 经 path 依赖自动纳入全部子模块 crate
  (共 26 成员),`cargo test --workspace` 实际覆盖核心 crate 自身测试。

### 修复 — submodule 协议新增 `tool_execution_request` 变体导致编译失败

- **`app-core` reducer 编译错误（根因）**：submodule `reflect-protocol` 新增
  `EventMsg::ToolExecutionRequest(ToolExecutionRequestEvent)`（v1.3 SDK serve
  模式远程工具执行请求）后，`app-core/src/reducer/mod.rs` 的穷尽 match 未覆盖
  该变体，`cargo check` 报 E0004、整个后端无法编译。已将其加入协议层
  no-op 分支（Desktop 内嵌 AgentThread、不注册远程工具，不会收到此事件）。
- **前端协议类型同步**：`EventMsgType` 联合此前缺 `tool_execution_request`
  （线格式 snake_case tag），已补齐类型镜像 `ToolExecutionRequestPayload`
  （`call_id` / `tool` / `args`）+ `EventMsgByType` 条目；
  `reduceEvent` 加对应 no-op case，配合穷尽性守卫保持前后端事件表一致。
- 文档：`docs/PROTOCOL_BRIDGE.md` §3.3 工具事件表补 `tool_execution_request`
  行（标注 serve 模式语义与 Desktop no-op 决策）。

### 修复 — 全量测试发现的问题（会话归属 / 事件层前向兼容 / 删除会话）

- **session 归属按内嵌 `session_meta.session_id` 匹配（根因）**：真实数据中存在
  「文件名 ≠ 内嵌 meta id」的会话文件（如 resume 沿用旧文件名写入新线程），
  而索引 `list_sessions` 以 meta id 标识 session，按文件名定位会全部 miss。
  `src-tauri/src/commands/sessions.rs` 统一改为：文件定位先读首行 meta
  （`first_line_session_id`），无 meta 时回退文件名 stem——三处受益：
  - 标题精化 `derived_titles` 以 meta id 为 key（此前按文件名 key，
    实测 260 个会话全部停留在 JSON 脏标题 `[{"text":"…","type":"text"}]`，
    精化从未生效；修复后脏标题清零，GUI 实机复验通过）；
  - 回放/导出兜底 `session_files` 同样按 meta 命中错位文件；
  - `reflect_delete_session` 此前删 `sessions/<id>/` 目录——真实布局是
    `YYYY/MM/DD/<id>.jsonl`，删除永远是静默 no-op；现删除该 session 的
    全部文件（含错位/轮转副本）+ `_names/<id>.name` + 旧布局目录。
- **事件层前向兼容**：
  - `reduceEvent`（`src/stores/agent/reducer.ts`）switch 无兜底分支，
    运行时遇到未知 `msg.type`（submodule 升级新增事件、前端类型未同步）
    返回 `undefined`，调用方 `Object.keys(patch)` 抛 TypeError 中断事件
    处理。新增带 `never` 穷尽性守卫的 `default` 分支：运行时安全返回空
    patch，同时保留「新增事件类型未处理时编译报错」的信号。
  - 前端协议类型镜像补齐 `plan_draft_updated`（`PlanDraftUpdatedEvent`，
    后端已有而前端联合缺失）；reducer 加对应 no-op case（GUI 决策路径
    走 `plan_ready` 弹窗，草稿正文暂不渲染）。
- 测试：sessions.rs 新增 4 个回归用例（meta 错位定位/精化、无 meta 回退、
  删除三布局）；agentStore.test.ts 新增未知事件安全 + plan_draft_updated
  用例。Rust 79 + 前端 564 用例全绿。

### 修复 — 历史会话无法显示 + 会话标题 + Composer 默认发送键

- **历史会话回放修复（根因 ×2）**：
  - 前端 `turnsFromRollout`（`src/stores/replay.ts`）此前按一个不存在的
    `{ seq, kind, payload }` 事件信封解析回放记录，而
    `reflect_replay_session` 实际返回的是 `reflect_protocol::RolloutRecord`
    tagged union（`{"type":"message","turn_id","role","content"}`）——
    所有记录都被丢弃，历史会话永远空白。已重写转换器：按 `turn_id` 分组，
    `content` 支持纯字符串与 `ContentBlock` 数组（text / tool_use /
    tool_result），`compaction` 记录渲染为单行 `compacted` 摘要（与 live
    `context_compacted` 一致）。
  - 后端 `reflect_replay_session` / `reflect_export_session` 兜底：
    submodule 的 `reader::replay` 只查「今天/昨天/前天」三日窗口，三日以上
    的历史会话拿到空结果。`src-tauri/src/commands/sessions.rs` 新增
    `replay_session()`：快路径为空时全树扫描该 session 的所有文件
    （跨日期目录 + 轮转副本 `.1`~`.3`，按时间升序拼接），导出命令同步受益。
- **会话列表标题**：
  - `SessionInfo.title`（自定义名 / 首条 user 消息派生）此前未被前端使用，
    侧边栏固定显示 session_id 前 8 字符。`displayTitle`（`buckets.ts`）
    现优先展示 `title`，无标题时回退 id 前缀；`ReflectSessionInfo`
    （`src/utils/types.ts`）补齐 `title` / token 字段镜像。
  - 后端标题精化（`src-tauri/src/commands/sessions.rs::refine_session_titles`）：
    submodule 派生标题对 block 数组 `content`（user 消息新格式）会退化成
    整段 JSON 字符串（`[{"text":"…","type":"text"}]`），且从不合并
    `_names/<id>.name` 自定义名。列表命令现做两级精化：自定义名优先
    （镜像 TUI 语义）→ block-aware 首条 user 消息重派生（全树单次 walk）。
  - 新会话首条消息发送后自动 invalidate 会话列表 query，派生标题立即可见
    （不再等 5min staleTime）。
- **Composer 默认发送键**：裸 Enter 现在直接发送（Shift+Enter 换行；
  ⌘/Ctrl+Enter 保持兼容；IME composition 期间的 Enter 不发送）。
  发送按钮 tooltip 文案同步更新（en/zh-CN）。

### 新增 — 前端 token usage UI 接通

后端 `token_count` 事件（`TokenCountEvent`）早已由 `model_call` 节点每轮 emit，前端
reducer 也已写入 `state.tokens`，但此前没有任何 UI 消费这些数据。本次接通展示层：

- **StatusBar**（`src/features/shell/StatusBar.tsx`）：右侧新增 token/cost 指示器，
  显示 `total` + `cost`（`testid="statusbar-tokens"`），hover tooltip 展示完整细分
  （input/output/cached/cacheWrite/total/cost）。
- **Inspector**（`src/features/shell/Inspector.tsx`）：新增 "Token Usage" section，
  镜像 MCP/LSP section 结构，展示 input / output / cached（含 "input 子集" 提示）/
  cacheWrite（>0 时显示，含 "不计入 total" 提示）/ total（强调）/ cost / provider /
  credential。空态显示 "No token data yet."
- **字段补全**：reducer 此前丢弃了 `cache_write_tokens` / `provider` /
  `credential_label`，现已对齐后端 `TokenCountEvent`；同时 `session_configured` 的
  `context_window_size` 也存入 `state.contextWindowSize`（为后续 usage 占比预留）。
- **斜杠命令引导**：`/usage` / `/cost` / `/context` / `/stats` / `/insights` /
  `/ctx_viz` 从 no-op stub 改为返回引导提示（指向 StatusBar/Inspector），保持
  `executeSlash` 纯函数契约。
- **i18n**：`inspector.*` 命名空间新增 token 相关文案（中英双语）。
- **测试**：`agentStore.test.ts` 补全 token_count + session_configured 字段断言；
  新增 `Inspector.test.tsx` / `StatusBar.test.tsx` 组件测试。
- **Tooltip 多行支持**：design-system `Tooltip` primitive 新增可选 `multiline`
  prop（CSS `white-space: pre-line`），让 StatusBar token tooltip 的
  `\n`-joined 细分真正换行渲染；默认关闭，不影响现有单行 tooltip。

> 注：未接入 `ContextRing` 圆环——后端无 token **分类**细分（仅有计费维度），硬套
> 5 段分类会制造虚假精确感；列为未来增强（需后端先提供分类 usage）。

### 新增 — Phase 3 纯本地功能批次（KMS + Autopilot + 听写接线）

补齐所有剩余的纯本地功能，不依赖云服务、SSO 或第三方 API。IM 桥接（Phase 2 第 7 项）
与多租户/SSO/云（Phase 3 第 14 项）明确跳过，因为它们依赖外部服务。

- **Phase 2 第 6 项：听写（Dictation）** — UI + hook + i18n + 路由此前已完成；
  补上缺失的 ActivityBar 入口（`/dictation`，Mic 图标）+ `shell.nav.dictation`
  i18n 键（en/zh-CN）。
- **Phase 3 第 12 项：KMS（知识管理系统）**：
  - **新 `app-core::kms` 模块**（`app-core/src/kms.rs`）：`KnowledgeManager`，提供
    基于 grep 的 wiki 存储（`~/.reflect/kms/<name>/pages/*.md`）、frontmatter
    解析（title/tags/description）、全文搜索和 `/dream` 会话挖掘。
    6 个单元测试全部通过。
  - **后端命令**（`src-tauri/src/commands/kms.rs`）：8 个命令 —
    `reflect_kms_list` / `reflect_kms_create` / `reflect_kms_delete` /
    `reflect_kms_save_page` / `reflect_kms_get_page` / `reflect_kms_list_pages` /
    `reflect_kms_search` / `reflect_dream`。
  - **前端包装**（`src/utils/commands/kms.ts`）：8 个函数 + 4 个类型。
  - **功能 UI**（`src/features/kms/`）：`KmsView.tsx`，含 wiki 选择 tab、
    页面列表、内联编辑器和全局搜索栏。
  - **状态接线**：`MinimalAgentInner.kms_manager` + facade `kms_manager()`。
- **Phase 3 第 10 项：Autopilot**：
  - **新 `app-core::autopilot` 模块**（`app-core/src/autopilot.rs`）：
    `AutopilotManager`，基于 JSON 的配置持久化 + 运行历史。
    `AutopilotConfig`（enabled/schedule/taskTemplate/agent/maxConcurrent）+
    `AutopilotRun` + `AutopilotRunStatus`。4 个单元测试全部通过。
  - **后端命令**（`src-tauri/src/commands/autopilot.rs`）：3 个命令 —
    `reflect_get_autopilot_config` / `reflect_update_autopilot_config` /
    `reflect_autopilot_history`。
  - **前端包装**（`src/utils/commands/autopilot.ts`）：3 个函数 + 2 个类型。
  - **功能 UI**（`src/features/autopilot/`）：`AutopilotView.tsx`，含配置
    编辑表单（enable/schedule/template/agent/concurrency）+ 运行历史面板。
  - **状态接线**：`MinimalAgentInner.autopilot_manager` + facade `autopilot_manager()`。
- **路由 + 导航**：注册 `/kms` + `/autopilot` 路由；
  `ActivityBar` 新增 KMS（BookOpen 图标）和 Autopilot（Zap 图标）入口；
  i18n `shell.nav.kms` + `shell.nav.autopilot`（en/zh-CN）。
- **验证**：`cargo check` ✅、`cargo test -p reflect-app-core` ✅（42 通过）、
  `pnpm typecheck` ✅、`pnpm test` ✅（61 文件 / 524 测试）。

### 新增 — Phase 3 第 8/9/11/13 项（Squad + Activity + Actor + Media Studio）

最后一批纯本地功能：补齐 Phase 3 路线图中所有不依赖云服务、SSO、第三方 API
或未经验证的外部桌面自动化 crate 的条目。

- **Phase 3 第 8 项：多态 Actor（语义层）** —
  - `app-core/src/actor.rs`：`ActorType`（Human / Agent / System）、
    `ActorKind`（User / Lead / Member / System）、`Actor { actor_type,
    actor_id, kind, display_name, team_name }`（camelCase serde）。
  - 辅助方法：`Actor::user()` / `system()` / `agent(team, role)` /
    `from_agent_id()` + `actor_from_team_member(&TeamMemberSpec)` +
    `encode_metadata` / `decode_metadata`，用于经 `Task.metadata.actor`
    往返序列化。不重构 vendor `Task` schema。
  - 12 个单元测试覆盖 `actor_type` / `actor_kind` 序列化、lead
    角色检测、metadata 往返、默认 = user。
- **Phase 3 第 9 项：Inbox + Activity 时间线 + @提及** —
  - `app-core/src/activity.rs`：`ActivityLogger`，内存环形缓冲（上限 500）+
    JSONL 持久化到 `~/.reflect/activity/`（1 MB 轮转）+ `record` /
    `list(filter)` / `search_mentions(q)` / `clear_memory`。
    `ActivityEvent { id, ts_ms, kind, actor, summary, task_id?, team_name?,
    level }`；`ActivityFilter` 按 kind / level / actor_id / team_name /
    since_ms 匹配。
  - `src-tauri/src/state/activity.rs`：独立的 broadcast 订阅者，
    将每条 `reflect_protocol::Event` 映射为 `ActivityEvent`
    并经 logger 写入。与 `forward_agent_events` 并行运行
    （互不干扰）。
  - `src-tauri/src/commands/activity.rs`：4 个命令 —
    `reflect_list_activity` / `reflect_search_activity` /
    `reflect_clear_activity` / `reflect_activity_count`。
  - `src/utils/commands/activity.ts`：TypeScript 包装 + `Actor` /
    `ActivityKind` / `ActivityLevel` 类型 + `extractMentions(text)`
    辅助函数（regex `@[a-z0-9_@-]+`）。
  - `src/utils/commands/squad.ts`：re-export。
  - `src/features/notifications/useActivityController.ts`：TanStack
    Query 控制器（activity + mentions 子查询）。
  - `src/features/notifications/NotificationsView.tsx`：重构为
    经 `SegmentedControl` 切换的三个 tab — **Inbox**（既有实时
    store 派生的 pending 项）、**Activity**（带 level 过滤 + 清除
    按钮的审计时间线）、**Mentions**（`@<query>` 输入框 +
    搜索结果）。
  - `ActivityLogger` 注入 `MinimalAgentInner` +
    `subscribe_activity_logger` 在 `install_agent_thread`
    中启动（线程之后、cron 调度器之前）。
  - 12 个 activity 单元测试（环形淘汰、过滤、持久化
    往返、mention 搜索）。
- **Phase 3 第 11 项：Squad + Leader 委派** —
  - `app-core/src/squad.rs`：`SquadSpec`（name、description、
    leader_actor、members、created_at_ms）+ `SquadMember { actor,
    role, model, system_prompt, allowed_tools }` + 包装
    `Arc<reflect_task::TaskManager>` 的 `SquadManager`。方法：
    `create_squad(spec)` 以 `TeamFile` upsert（`team-lead@<name>`
    lead + 映射后的 member spec）；`list_squads()` / `get_squad(name)` /
    `delete_squad(name)`；`delegate_next(squad_name, leader_id)`
    调用 vendor `claim_next_available`，以 per-list mutex 安全地
    原子认领首个 Pending + 已解除阻塞的任务；
    `assign_task(squad_name, task_id, assignee)` 经 vendor `TaskPatch`
    写入 `owner` + `metadata.actor`。
  - 不重构 vendor Team/Task schema；squad 通过既有的
    `~/.reflect/teams/<name>.json` 文件存储。
  - `src-tauri/src/commands/squad.rs`：6 个命令 —
    `reflect_list_squads` / `reflect_create_squad` /
    `reflect_get_squad` / `reflect_delete_squad` /
    `reflect_delegate_next` / `reflect_assign_squad_task`。
  - `src/utils/commands/squad.ts`：包装 + `ReflectSquadSpec` /
    `ReflectSquadMember` 类型。
  - `src/features/squad/`：`SquadView`（master-detail：左侧 squad 列表
    + 创建表单，右侧所选 squad 的 members + tasks + "Delegate next"
    按钮 + 每任务 assignee 下拉）+ `useSquadController`（TanStack Query，
    含 create / delete / delegate / assign mutations）+ CSS module +
    barrel + 3 个测试。
  - 12 个 squad 单元测试（CRUD、validate、spec↔TeamFile 往返、
    delegate、带 actor metadata 往返的 assign）。
- **Phase 3 第 13 项：Media Studio + Computer Use（仅元数据
  脚手架）** —
  - `app-core/src/media.rs`：`MediaAsset`（path / filename /
    size_bytes / mime_type / width / height / modified_at_ms）+
    `ImageProcessSpec` / `ImageProcessResult` + `ImageFormat`
    （Png / Jpeg / Gif / WebP / Bmp）+ `scan_dir_for_assets(dir)`
    （基于 stdlib，无外部依赖）+ `BackendCapability` enum +
    `ImageBackend` / `ComputerBackend` traits +
    `ComputerUseAction`（Screenshot / MouseMove / MouseClick /
    KeyType / KeyCombo / Scroll，`#[serde(tag = "kind")]`）。
  - 默认后端：`MetadataOnlyBackend`（仅读文件头）+
    `UnavailableComputerBackend`（返回
    `MediaError::Unavailable` 并附 capability 原因）。需要真实
    `image` / `xcap` / `enigo` cargo 依赖的命令会优雅地
    返回 `MediaError::Unavailable` 而非 panic，
    因此该功能在 dev / CI / headless 下可用。
  - `src-tauri/src/commands/media.rs`：5 个命令 —
    `reflect_list_media` / `reflect_image_process` /
    `reflect_screenshot`（base64 编码的 PNG 字符串）/
    `reflect_computer_use` / `reflect_media_capabilities`。
  - `src/utils/commands/media.ts`：包装 + 类型
    （`ReflectMediaAsset` / `ReflectImageProcessSpec` /
    `ReflectComputerUseAction` / `ReflectMediaCapabilities`）。
  - `src/features/media/`：`MediaView`（经 `SegmentedControl` 的
    2 个 tab — **Studio** 列出用户输入目录中的 asset；
    **Computer Use** 提供 screenshot / click / move / scroll /
    keytype / key combo 控制卡片，并优雅显示
    `MediaError::Unavailable`）+ `useMediaController`（TanStack
    Query）+ CSS module + barrel + 3 个测试。
  - 12 个 media 单元测试，覆盖格式检测、Asset 序列化、
    action 摘要、后端错误路径和 `scan_dir` 过滤。
  - **注**：真实的图像处理 / 截屏 / 鼠标键盘控制需要
    `image` / `xcap` / `enigo` cargo crate。本批次有意
    不将它们加入 `Cargo.toml` —— 契约是稳定的，
    把默认后端换成真实实现只是局部改动。
- **路由 + 导航**：注册 `/squad` + `/media` 路由；
  `ActivityBar` 新增 Squad（Users 图标）+ Media（Image 图标）
  入口；i18n `shell.nav.media`（en/zh-CN）。`shell.nav.squad`
  已在听写批次中加入。
- **验证**：`cargo check` ✅、`cargo test -p reflect-app-core`
  ✅（90 通过；12 actor + 12 activity + 12 squad + 12 media + 42
  既有）、`pnpm typecheck` ✅、`pnpm test` ✅（63 文件 / 530
  测试）。

### 新增 — Phase 2 第 2 项：远程 daemon / Tailscale 助手 / iOS 配置 UI

Phase 2 首个条目：用户驱动的并发 agent 编排（相对于模型驱动的 `Task`
subagent）。每个 side-channel 在自己的 `CancelToken` 上与主 agent 并发
运行 —— 主 agent 的 `Cmd+C` 不会停止它。

- **新 `app-core::side_channel` 模块**（`app-core/src/side_channel.rs`）：
  - `SideChannelRegistry`（进程级）+ `SideChannelHandle`（每次运行）。
  - 稳定 id `side-<8hex>`，冲突时确定性 salt 重试。
  - 经 `tokio::sync::broadcast::Sender<SideChannelEvent>` 发出
    `Started` / `Done` / `Cancelled` / `Error` / `Output` 事件 —— 既有
    Tauri 事件转发器可接收，并以 `reflect_event` 消息（标记
    `kind: side_channel_*`）呈现给前端。
  - 6 个单元测试（start 返回 id + cancel token / cancel 发出事件 /
    对已终结的取消是 noop / finish done/error 状态迁移 /
    独立 cancel / cancel 不阻塞后续 start）。
  - `app-core/Cargo.toml` 新增 `tokio-util = { workspace = true, features = ["rt"] }`
    以支持 `CancellationToken`。
- **后端接线**：
  - `MinimalAgentInner.side_channels` 持有 `SideChannelRegistry`（在
    `build_empty_inner` 中构建，故安装前即就绪）。
  - 供命令层使用的 facade `MinimalAgent::side_channels()`。
  - `src-tauri/src/commands/side_channel.rs` 新命令：
    `reflect_start_side_channel` / `reflect_cancel_side_channel` /
    `reflect_list_side_channels` / `reflect_get_side_channel`。
  - 6 个单元测试覆盖 start/get/cancel + finish 状态迁移。
- **前端包装**（`src/utils/commands/side_channel.ts`）：
  - 4 个函数 + `ReflectSideChannelInfo` / `ReflectSideChannelStatus` /
    `ReflectStartSideChannelResult` 类型。
  - Index barrel 新增 re-export；`commands.test.ts` 增加 3 条转发
    断言（list/get 无参数、start 转发 `{ agent_name, prompt }`、
    cancel 转发 `{ id }`）。
- **功能 UI**（`src/features/side-channel/`）：
  - `SideChannelView.tsx` — PageShell + 运行计数徽标 + Start 表单
    （agent 名 + prompt）+ 列表行（id / agent / prompt / 时长）+
    运行行上的取消按钮。
  - `useSideChannelController.ts` — TanStack Query + start/cancel
    mutations，带 toast + 5s 轮询 refetch。
  - 5 个冒烟 + 行为测试。
- **路由 + 导航**：注册 `/side-channels` 路由；`ActivityBar`
  新增 Side-channels 入口（`Workflow` 图标）；i18n
  `shell.nav.sideChannels`（en/zh-CN）。
- **范围说明**（后续计划）：真正运行 side-channel 的运行时 driver
  （把 prompt 作为 `Submission::user_input` 提交到 agent
  循环并在完成后更新注册表条目）尚未实现。
  当前视图展示注册表状态并提供 create/cancel；
  driver 是 Phase 2 下一个子项。
- **验证**：`cargo check` ✅、`cargo test commands` ✅（27 通过）、
  `pnpm typecheck` ✅、`pnpm test` ✅（60 文件 / 516 测试）。

### 新增 — Phase 2 第 2 项：远程 daemon / Tailscale 助手 / iOS 配置 UI

Phase 2 第二个条目：桌面 daemon 状态呈现 + Tailscale 网络助手
+ iOS 配置入口。为远程客户端（iOS）经 Tailscale 连接到
ReflectDesktop 实例提供建设基础。

- **新 `app-core::tailscale` 模块**（`app-core/src/tailscale.rs`）：
  - `TailscaleStatus` 结构体（installed/running/dns_name/ipv4/ipv6/suggested_remote_host）。
  - `detect()` — shell 调用 `tailscale status --json=true`，解析 JSON 输出，
    任何失败（缺二进制、非零退出码、解析错误）都返回降级状态。
  - `daemon_command_preview()` — headless daemon 配置的提示字符串。
  - `derive_suggested_host()` — 优先 DNS 名（如 `node.tail.net`），回退到 IPv4。
  - 6 个单元测试（4 同步 + 2 异步）：建议 host 推导（DNS/IPv4/无）、
    降级状态形状、daemon 命令预览含默认端口、
    即使 tailscale 缺失 detect 也返回状态而不 panic。
- **远程配置状态**（`src-tauri/src/state/remote_config.rs`）：
  - `RemoteConfig`（host/port/auth_token/auto_connect）+ 返回
    `<host>:<port>` 字符串的 `endpoint()` 方法 + 检查 host 是否已设置的
    `is_ready()`。
  - `RemoteStatus`（state/message/endpoint/since_ms）用于连接跟踪。
  - 默认配置：host/port/auth_token/auto_connect 全为 null 或空。
  - 4 个单元测试：默认 endpoint 使用默认端口、设置 host 后
    is_ready 为真、host 为空时 is_ready 为假、断开状态携带 since_ms。
- **后端接线**：
  - `MinimalAgentInner.remote_config: RwLock<RemoteConfig>`（在
    `build_empty_inner` 中以默认配置构建）。
  - 供命令层使用的 facade 访问器 `MinimalAgent::remote_config()`。
  - `src-tauri/src/commands/remote.rs` 新命令：
    `reflect_get_remote_config` / `reflect_update_remote_config` /
    `reflect_get_remote_status` / `reflect_tailscale_status` /
    `reflect_tailscale_daemon_command_preview` /
    `reflect_tailscale_daemon_start` / `reflect_tailscale_daemon_stop` /
    `reflect_tailscale_daemon_status`。
  - `RemoteConfigSnapshot`（camelCase serde）携带 endpoint + is_ready 标志。
  - 3 个单元测试：snapshot endpoint/ready 标志处理、端口默认、
    更新配置往返。
- **前端包装**（`src/utils/commands/remote.ts`）：
  - 8 个函数 + `ReflectRemoteConfigSnapshot` / `ReflectRemoteStatus` /
    `ReflectTailscaleStatus` 类型。
  - Index barrel re-export；`commands.test.ts` 增加 8 条转发断言
    （带默认值的 config get/update、无参数的 tailscale status、
    无参数的 daemon 命令 preview/start/stop/status）。
- **功能 UI**（`src/features/remote/`）：
  - `RemoteView.tsx` — PageShell + 4 个卡片区块：iOS 配置（host/port/auth/
    auto_connect 编辑表单 + 保存）、Tailscale 检测（状态徽标 + DNS 名 +
    IPv4 显示）、Daemon 提示（命令预览 + 复制到剪贴板）、传输状态
    （断开/连接状态显示）。
  - `useRemoteController.ts` — 4 个 TanStack Query 查询（remote config、
    remote status、tailscale status、daemon 命令预览）+ update mutation
    + iOS 配置表单的草稿编辑状态。
  - `RemoteView.module.css` — 纯 token 卡片布局样式。
  - 5 个测试：页面标题、4 张卡片可见、ready 徽标 + endpoint 显示、
    tailscale 字段渲染、daemon 预览文本展示、保存转发 update。
- **路由 + 导航**：注册 `/remote` 路由；`ActivityBar`
  新增 Remote 入口（`Wifi` 图标）；i18n `shell.nav.remote`（en/zh-CN）。
- **范围说明**（后续计划）：真正的 TCP JSON-RPC daemon 二进制
  （`src-tauri/src/bin/reflect_daemon.rs`）尚未实现 —— 需要
  独立的 workspace crate 和交叉编译目标。iOS 客户端
  应用也尚未启动。当前实现先提供桌面配置
  界面与 Tailscale 网络检测作为前置。
- **验证**：`cargo check` ✅、`cargo test -p reflect-app-core -- tailscale`
  ✅（6 通过）、`cargo test -p reflect-desktop -- remote` ✅（7 通过）、
  `pnpm typecheck` ✅、`pnpm test` ✅（61 文件 / 524 测试）。

### 新增 — Phase 1 第 3 项：Agent 定义管理（profile UI）

落地 Phase 1 第 3 项的最后一个切片：端到端的 agent profile 管理。
现在可以在桌面端创建 / 编辑 / 删除 agent 定义，
以 Markdown + YAML frontmatter 形式存储于 `~/.reflect/agents/<name>.md`，
与 TUI/CLI 共享。

- **依赖**：`src-tauri/Cargo.toml` 新增
  `reflect-agent-def = { workspace = true }` + `serde_yaml`
  （serde_yaml 用于保存时的 frontmatter 序列化；
  vendor crate 仅提供解析器）。
- **后端命令**（`src-tauri/src/commands/agents.rs`）：
  - `reflect_list_agent_defs` / `reflect_get_agent_def` /
    `reflect_save_agent_def` / `reflect_delete_agent_def` /
    `reflect_parse_agent_md`（无副作用的预览/校验）。
  - 保存时序列化 frontmatter（YAML，省略空的 optional 字段）+ body，
    然后重新解析写入的文件做往返校验。
  - 路径安全：`path_for()` 拒绝名称中的空 / `..` / 斜杠 / 反斜杠 /
    NUL，防止越出 `~/.reflect/agents/` 的路径穿越。
  - 错误映射：`From<AgentDefError> for CommandError`。
  - 4 个单元测试：序列化往返、minimal 省略 optional、
    路径穿越拒绝、文件名构建。
- **前端包装**（`src/utils/commands/agents.ts`）：5 个函数 +
  `ReflectAgentDef` / `ReflectMemoryScope` 类型。`index.ts` re-export；
  `commands.test.ts` 中 4 条转发断言。
- **功能 UI**（`src/features/agents/`）：
  - `AgentsView.tsx` — PageShell + 列表行（name / description / model /
    readonly / spawnable 徽标）+ New 按钮。
  - `AgentEditor.tsx` — 全字段编辑器：name / description / model /
    tools（csv）/ disallowed_tools（csv）/ spawnable / readonly /
    max_turns / max_result_chars / memory scope / system_prompt（markdown
    body）。编辑时锁定名称（重命名会改变文件）。
  - `useAgentsController.ts` — query + save/delete mutations + 草稿状态。
  - 5 个冒烟 + 行为测试。
- **路由 + 导航**：注册 `/agents` 路由；`ActivityBar` 新增
  Agents 入口（`Bot` 图标）；i18n `shell.nav.agents`（en/zh-CN）。
- **验证**：`cargo check` ✅、`cargo test commands` ✅（21 通过）、
  `pnpm typecheck` ✅、`pnpm test` ✅（59 文件 / 508 测试）。

### 新增 — Phase 1 第 2 项：Schedule（cron）命令面 + UI

端到端落地 cron 驱动的自主触发（后端 → 包装 → UI）。vendor
`reflect-stream::cron::CronScheduler` 现已接入桌面端：
`install_agent_thread` 注入真实的 `AgentThread::submission_sender()` 并
启动 30s driver tick；到期的 job 会把其 `prompt` 作为
`Submission::user_input` 发送到 agent 循环。

- **依赖**：`src-tauri/Cargo.toml` 新增
  `reflect-stream = { workspace = true }`。
- **状态注入**：`MinimalAgentInner.cron_scheduler:
  RwLock<Option<CronScheduler>>`（安装前为 None）。facade
  `MinimalAgent::install_cron_scheduler(sender)` 用真实 sender 构建调度器，
  迁移既有 job，启动 30s driver 并写回。
  供命令层使用的 `cron_scheduler()` 访问器。
- **后端命令**（`src-tauri/src/commands/schedule.rs`）：
  - `reflect_list_schedules` / `reflect_add_schedule` /
    `reflect_update_schedule` / `reflect_remove_schedule` /
    `reflect_get_schedule_status`。
  - 错误映射：新增 `From<CronParseError> for CommandError`；`thiserror`
    `Display` 保留 variant 信息。
  - 3 个单元测试覆盖 create/list/update/delete 链、非法表达式
    拒绝和错误映射。
- **前端包装**（`src/utils/commands/schedule.ts`）：5 个函数 +
  `ReflectCronJob` / `ReflectScheduleStatus` 类型（snake_case vendor payload；
  camelCase 状态信封，对应 Rust `rename_all`）。`index.ts`
  新增 re-export；`commands.test.ts` 增加 4 条转发断言。
- **功能 UI**（`src/features/schedule/`）：
  - `ScheduleView.tsx` — PageShell + 状态徽标（`enabled/total active`）+
    内联创建表单 + job 行（schedule / prompt / next-fire + 开关 /
    移除）。
  - `useScheduleController.ts` — list + status 的 TanStack Query + 3 个
    mutations（add / toggle / remove），带 toast + invalidation。
  - `ScheduleView.module.css` — 仅用 design token。
  - 6 个冒烟 + 行为测试。
- **路由 + 导航**：注册 `/schedule` 路由；`ActivityBar` 新增
  Schedule 入口（`Clock` 图标）；i18n `shell.nav.schedule`（en/zh-CN）。
- **范围外（下一步）**：`run_now`（需要 vendor `pub async fn run_now(&self)`
  —— `CronScheduler::tick` 是 public 但读取私有的 `jobs` Arc；上游一行
  访问器是干净路径）、持久化（vendor 调度器是
  内存态；重启丢 job）、tick 间隔配置、一次性 `run_at`。
- **验证**：`cargo check` ✅、`cargo test commands::schedule` ✅（3/3）、
  `pnpm typecheck` ✅、`pnpm test` ✅（58 文件 / 499 测试）。

### 新增 — Phase 1 多 agent 功能 UI（Tasks 看板）

端到端补全 Phase 1 第 1 项（「Team/Task/Coordinator 命令面」）：
后端命令（上上个切片）和 IPC 包装（上个切片）现在可以在桌面功能
视图中驱动。多 agent 任务协调首次在 GUI 中可见、可操作。

- **新功能** `src/features/tasks-board/`：
  - `TasksBoardView.tsx` — 页面外壳，含 List / Board 视图切换
    （`SegmentedControl`）、活跃列表选择器（自由文本 list id + 来自
    `reflect_list_teams` 的 team chip）和内联创建表单。
  - `useTasksBoardController.ts` — `reflect_list_tasks` /
    `reflect_list_teams` 的 TanStack Query + 四个 mutations（create /
    claim / advance-status / delete），带 toast + 缓存失效。沿用
    `useMemoryController` 的形状（2026-07-25 重构先例）。
  - `TaskRow.tsx` — 展示性行，含 id / subject / claimer / 状态
    徽标 + 按状态的操作（Claim / Start / Complete / Delete）。
  - `TaskCreateForm.tsx` — 内联创建表单（subject / description / owner）。
  - `TasksBoardView.module.css` — 仅用 design token 的样式（无 inline
    hex），board 视图是 3 列网格，900px 以下折叠为 1 列。
  - `index.ts` — feature barrel。
- **路由**：`tasksRoute`（`/tasks`）注册到 `src/router.tsx`。
- **导航**：`ActivityBar` PRIMARY 分组新增 Tasks 入口
  （`FolderKanban` 图标）；新增 i18n 键 `shell.nav.tasks`（en/zh-CN）。
- **测试**：`TasksBoardView.test.tsx` — 8 个冒烟 + 行为用例，覆盖
  空态、创建表单提交、board 视图切换、带操作按钮的预置行，
  以及 claim / complete / delete 转发到正确命令。
  全套：57 文件 / 489 测试通过。
- **范围外（下一步）**：Phase 1 第 2 项 —— Schedule（cron）命令面
  （后端 `commands/schedule.rs` + 包装 `commands/schedule.ts` + UI）。

### 新增 — Phase 1 多 agent IPC 包装（前端）

为上个切片落地的 Task/Team 命令提供前端 TypeScript 包装。
这 10 个后端命令现在可从
`@/utils/commands`（以及兼容 barrel `@/utils/commands` /
`@/utils/tauri`）调用。

- **新包装**：
  - `src/utils/commands/tasks.ts` — `reflect_list_tasks` /
    `reflect_create_task` / `reflect_get_task` / `reflect_update_task` /
    `reflect_claim_task` / `reflect_delete_task` + 类型 `ReflectTask`、
    `ReflectTaskStatus`、`ReflectTaskPatch`、`ReflectTaskUpdateResult`、
    `ReflectTaskStatusChange`。
  - `src/utils/commands/teams.ts` — `reflect_list_teams` /
    `reflect_upsert_team` / `reflect_get_team` / `reflect_delete_team` +
    类型 `ReflectTeam`、`ReflectTeamMember`。
- **类型保真**：`Task` / `TeamFile` / `TeamMemberSpec` 逐字对应
  vendor serde 形状（snake_case，因为 Rust 类型未
  derive `rename_all = "camelCase"`）。只有 `TaskUpdateResult` 信封是
  camelCase（对应 `commands/tasks.rs` 中的 Rust
  `#[serde(rename_all = "camelCase")]`）。`TaskPatch` 用
  `T | null | undefined` 镜像 `Option<Option<T>>`
  三态。
- **Barrel 接线**：`src/utils/commands/index.ts` re-export 两个模块
  （置于 `hooks` 之后、`git` 之前）。兼容 barrel
  （`src/utils/commands.ts`、`src/utils/tauri.ts`）经既有的
  `export * from './commands/index'` 链自动获得。
- **测试**：`src/utils/commands.test.ts` 增加 8 条转发断言，
  锁定全部 10 个新包装的命令名 + 参数键不变量
  （沿用既有 per-domain 模式）。全套：
  56 文件 / 481 测试通过。

### 新增 — Phase 1 多 agent 命令面（Task / Team）

后端 `vendor/reflect-task` 早已实现完整的 `TaskManager` API
（Task/Team CRUD + 原子认领 + 依赖跟踪），但它既不是 Tauri 应用的
依赖也未暴露为命令。本次落地多 agent 桌面路线图
（`docs/reference-projects-survey.md` §10 Phase 1 第 1 项）的
第一个切片：把 `TaskManager` 接入 `MinimalAgentInner` 并暴露
10 个命令，让前端可以观察 / 驱动多 agent 协同。

- **依赖**：`src-tauri/Cargo.toml` 新增
  `reflect-task = { workspace = true }`。存储复用 vendor 默认 home
  （`~/.reflect/tasks/<list>/`、`~/.reflect/teams/<name>.json`），与
  TUI/CLI 共享 —— 无新配置项，不改 vendor。
- **状态注入**：`MinimalAgentInner` 现持有
  `Arc<reflect_task::TaskManager>`（Phase 0 形态：无 `hook_engine` /
  `event_sink`；它们在前端 UI 订阅任务生命周期事件时落地）。
  新增 facade 访问器 `MinimalAgent::task_manager()`。
- **命令**（`src-tauri/src/commands/tasks.rs`，注册于
  `src-tauri/src/lib.rs::invoke_handler`）：
  - Task：`reflect_list_tasks` / `reflect_create_task` / `reflect_get_task` /
    `reflect_update_task` / `reflect_claim_task` / `reflect_delete_task`。
  - Team：`reflect_list_teams` / `reflect_upsert_team` / `reflect_get_team` /
    `reflect_delete_team`。
- **返回类型整形**：`reflect_update_task` 返回扁平化的
  `TaskUpdateResult { task, updatedFields, statusChange? }` 而非
  vendor `UpdateOutcome`（其 `(TaskStatus, TaskStatus)` 元组对前端
  不友好且类型缺 `Serialize`）。以 camelCase 序列化
  以匹配既有 IPC 约定。
- **错误映射**：`commands/error.rs` 新增
  `From<reflect_task::TaskError> for CommandError`；`TaskError` 的
  `thiserror::Display` 确保不丢失
  variant 信息。
- **测试**：`commands::tasks` 覆盖 task create→get→update→list→claim→
  delete、team upsert→get→list→delete 和错误映射（3 个测试，
  全绿）。不启动 Tauri 运行时（沿用 `commands/sessions.rs` 模式）。
- **范围外（后续轮次）**：前端 IPC 包装
  （`src/utils/commands/{tasks,teams}.ts`）、功能 UI
  （`src/features/{tasks-board,agents}/`）、Task 生命周期事件的
  `hook_engine`/`event_sink` 注入、Schedule（cron）命令、Coordinator
  模式开关。

### 变更 — 结构重构（规范化实时状态文档）

结构重构后的规范化实时状态文档整理。无代码或
运行时行为变更；仅更新文档。模块布局重写为
与重构后的文件树一致。移除过时的硬编码计数（Tauri 命令
数、命名空间数等），改用「按域拆分」表述。

- **后端 `src-tauri/src/commands/`**：每个 `#[tauri::command]` 函数体现在位于
  `src-tauri/src/commands/<domain>.rs` 的按域模块中；薄壳
  `src-tauri/src/commands/mod.rs` re-export 它们。共享错误助手位于
  `src-tauri/src/commands/error.rs`（`CommandError` / `CommandResult`）。域
  模块涵盖 agent / allowlist / config / export / files / git / hooks /
  memory / search / sessions / shell / skills / update / workspaces。完整
  命令清单枚举于 `docs/PROTOCOL_BRIDGE.md` §2.0 并注册在
  `src-tauri/src/lib.rs::invoke_handler`。

- **后端 `src-tauri/src/` 支撑模块**：从
  `state.rs` 拆出的私有助手现与其并列：`hook_store.rs`、`memory_store.rs`、
  `shell_sessions.rs` 和 `workspace_state.rs`。public 模块仍为
  `state.rs`、`events.rs`、`dock.rs`、`menu.rs`、`shortcut.rs`、`tray.rs` 和
  `mcp.rs`。

- **前端 agent store**：Zustand store 实现现为
  `src/stores/agent/` 模块（`store.ts`、`reducer.ts`、`turns.ts`、`toast.ts`、
  `servers.ts`、`types.ts`、`useAgent.ts`、`index.ts`）。`src/stores/agentStore.ts`
  保留为薄兼容 facade，re-export `useAgentStore`、
  `reduceEvent`、`useAgent` 和来自 `./agent` 的类型联合。新代码应
  直接从 `@/stores/agentStore` 导入（或从 `./agent` 获得更细
  粒度）；旧 `src/services/agent.ts` re-export 继续可用。

- **Composer 归属**：`Composer` 现由 `src/features/composer/` 持有
  （`Composer.tsx`、`SlashPopup.tsx`、`MentionPicker.tsx`、`AttachmentBar.tsx`、
  `slashCommands.ts`、`slashEngine.ts`、`useComposerInput.ts`、
  `useComposerSubmission.ts`、`useAttachments.ts`、`usePromptHistory.ts`）。原先的
  `src/features/messages/Composer.tsx` 现为薄 re-export 垫片 ——
  `export { Composer } from '@/features/composer/Composer'` —— 因此既有
  导入继续可用。

- **Settings 拆分**：`src/features/settings/` 拆为：
  - 顶层外壳 —— `SettingsView.tsx`、`ConfigForm.tsx`、`configSchema.tsx`
  - `sections/` —— `DisplaySection.tsx`、`NotificationsSection.tsx`、
    `UpdatesSection.tsx`（各自带适用的 `.ssr.test.tsx`）
  - `components/` —— `StructuredField.tsx`、`ComplexEditors.tsx`、`index.ts`
    （共享表单原子 + 复杂区块编辑器）
  - `config/` —— `schema.ts`（按 `ReflectConfig` 区块的 FieldSpec 目录）、
    `toml.ts`（保持未知键完整的纯读/写助手）、`index.ts`
    （barrel）、`toml.test.ts`

- **前端 IPC 包装**：按域包装现在位于
  `src/utils/commands/<domain>.ts`，由
  `src/utils/commands/index.ts` 聚合。`src/utils/tauri.ts` 和 `src/utils/commands.ts`
  保留为兼容 barrel，从 `./bridge` + `./commands`
  + `./types` re-export。`src/utils/bridge.ts` 是底层 `invoke` / `listen` + Tauri
  上下文回退层，带 `isMissingTauriInvokeError` 守卫。

- **i18n 拆分**：运行时拆为
  `src/utils/i18n/{context.tsx,locale.ts,interpolate.ts,lookup.ts,types.ts}`；
  `STRINGS` 字典在 `src/utils/i18n/strings/index.ts` 中由
  `src/utils/i18n/strings/` 下的按命名空间目录模块合并组成
  （about / app / apps / chat / collaboration / common / composer / debug /
  design / dictation / files / git / home / inspector / memory / mobile /
  modal / models / notifications / palette / permissionMode / plan / prompts /
  settings / shell / sidebar / skills / slash / terminal / threads / toast /
  update / workspaces）。`src/utils/i18n.ts` 是兼容 barrel，
  re-export 运行时 API 及合并后的 `STRINGS` / `ALL_KEYS`。

- **Modal 拆分**：`src/features/modals/` 现在每个 modal body 一个
  `.tsx` —— `ApprovalModal.tsx`、`QuestionModal.tsx`、`AskUserModal.tsx`、
  `PlanReadyModal.tsx`、`ApprovalHistory.tsx` —— 外加共享的 `ModalShell.tsx`
  和 `index.tsx` 的 `ModalStack` 编排器。

- **Terminal / memory / shell 拆分**：有状态编排抽为
  同目录的 controller：
  - `src/features/terminal/TerminalView.tsx`（展示层）+
    `src/features/terminal/useTerminalController.ts`（sessions / lines /
    run / kill / clear）
  - `src/features/memory/MemoryView.tsx`（展示层，含 `MemoryRow.tsx`
    和 `MemoryAddForm.tsx`）+ `src/features/memory/useMemoryController.ts`
    （TanStack query + mutations + 过滤/编辑/新表单状态）
  - `src/features/shell/hooks/{useCommandPaletteShortcut,usePaletteActions,useThemeCycle}.ts`
    承担 shell 层交互。

- **文档更新**：`AGENTS.md`、`README.md`、`docs/codebase-map.md`、
  `docs/ARCHITECTURE.md`、`docs/PROTOCOL_BRIDGE.md` 和 `docs/CHANGELOG.md`
  重写为以重构后布局为规范的实时状态。changelog 之外无
  历史评注。所有引用路径均存在。

### 修复 — 核心 UX

- **窗口拖拽区**：TitleBar 的 `-webkit-app-region: drag` 现在真正生效。在
  `src/styles/base.css` 中加固 `[data-tauri-drag-region]`（`position: relative; z-index: 1;
  user-select: none`），避免 flex/transform 祖先截获拖拽。在
  `src/features/shell/TitleBar.module.css` 中对 `.bar` / `.left` / `.right` / `.title` / `.sessionStatus`
  显式声明 `-webkit-app-region: drag` 作为双保险。放宽
  `AppShell.module.css` 的 `.shell { overflow: clip }`（原为 `hidden`），让拖拽元素
  真正收到 mousedown。TitleBar 增加 `onMouseDown` 守卫，防止 webview
  抢焦点破坏原生拖拽。

- **全项目 i18n（英语 + 简体中文）**：`src/utils/i18n.ts` 从
  32 个键扩展到 **280+ 个键**、覆盖 24 个命名空间（`common`、`app`、`shell`、`sidebar`、
  `threads`、`composer`、`chat`、`settings`、`palette`、`modal`、`home`、`files`、`git`、
  `terminal`、`skills`、`workspaces`、`models`、`plan`、`prompts`、`notifications`、
  `about`、`apps`、`collaboration`、`debug`、`mobile`、`update`、`memory`、`dictation`、
  `design`、`toast`、`slash`、`permissionMode`、`inspector`）。此前硬编码英文文案的 43 个组件
  全部改为经 `useI18n()` 渲染。标题、aria-label、
  toast、命令面板条目、slash 命令、听写语言（`zh-CN` /
  `en-US`）、权限模式描述、配置 schema 字段标签与
  占位符、全部四个 modal（Approval / Question / AskUser / PlanReady）、
  ConfigForm 每个区块、每个路由视图（Home / Files / Search / Git / Terminal / Skills
  / Workspaces / Models / Plan / Prompts / Notifications / About / Apps / Collaboration
  / Debug / Mobile / Update / Memory / Dictation / DesignSystem）全部本地化。目录
  一致性由 `src/utils/i18n.test.ts` 保证（一致性 + 插值 + 复数 +
  回退）。持久化到 `localStorage` 并在切换时同步到 `<html lang>`。

- 新增持久化的英/简体中文本地化，Settings 中提供语言选择器，覆盖核心 shell、聊天、会话和设置 UI。
- 用全结构化表单取代仅覆盖 provider 的窄表单，为 `vendor/reflect-config/src/schema.rs` 支持的每个区块渲染直接输入项：`active`、`anthropic`、`openai`、`ollama`、`compact`、`token_budget`、`sandbox`、`routing.{main,compact,subagent}`、`coordinator`、`ask_user_question`、`model`、`analytics`、`notifications`、`postgres_session`、`sse_redis`、`bridge`、`voice`、`dap`、`acp`、`sanitize`、`plugins`、`feature_flags`、`mcp_servers`、`lsp_servers`、`hooks`、`subagent_providers`、`config_version`。每个输入项就地编辑并序列化 TOML；高级 raw TOML 编辑器仍作为兜底出口。
- 会话选择现在会重放所选 rollout、替换过期聊天轮次，并显示本地化的加载、空态、重试和错误状态。通过 request id ref 做竞态保护。

### 新增

- **外观定制**：Settings → Display 现提供 system/dark/light 主题、自定义强调色、可调表面透明度、持久的本地或 URL 背景图以及背景图强度。外观偏好从既有 `reflect.uiprefs.v1` 数据安全迁移并经共享 design token 应用；开启「减少透明度」时强制不透明表面并隐藏壁纸。
- `ConfigForm` 组件（`src/features/settings/ConfigForm.tsx`）与 `configSchema` 助手（`src/features/settings/configSchema.tsx`）。
- `src/features/messages/ChatView.test.tsx` 覆盖加载、空态、错误、重试、语言切换和会话清除流程（5 个用例）。
- 扩展 `SettingsView.test.tsx`，覆盖全部 25+ 个结构化输入和 `configSchema` 助手（5 个新用例）。

### 新增 — Batch 12（原生菜单接线 + 文档打磨）

- **原生菜单基础设施**：`src-tauri/src/menu.rs` 已提供 5 个子菜单
  （Reflect / Edit / Composer / View / Window），含 12 个带快捷键的菜单项：
  - Reflect：About / Check for Updates / Settings (Cmd+,) / Quit
  - Edit：Undo / Redo / Cut / Copy / Paste / Select All（Predefined）
  - Composer：Cycle Model (Cmd+M) / Cycle Reasoning (Cmd+R) / New Agent (Cmd+N) / Interrupt (Cmd+.)
  - View：Toggle Sidebar (Cmd+B) / Toggle Terminal (Cmd+T)
  - Window：Minimize / Zoom / Close
- **菜单事件转发**：`handle_menu_event` 向前端发出 `menu-*` 事件，覆盖
  Settings 导航、Cycle Reasoning、New Agent、Interrupt、Toggle Sidebar、Toggle Terminal。
  AppShell 增加 menu 事件监听器，经 `listen()` 处理这些事件。
- **听写 stub**：`DictationView` 显示 "Voice input coming soon"，带 Mic 图标
  及 Web Speech API / macOS Speech Recognition 路线图。
- **更新视图**：`UpdateView` 显示来自 ping 查询的当前版本、经
  `bash scripts/install.sh` 的手动更新说明，以及 tauri-plugin-updater 集成备注。
- **TitleBar ⌘K 提示**：右侧 Search 图标 + "⌘K" 键盘提示（带 tooltip），
  让 CommandPalette 更易被发现。
- **StatusBar 会话计数**：来自 `reflect_list_sessions` 的 MessagesSquare 图标 + 会话数，
  tooltip 为 "N sessions on disk"。
- **文档**：CHANGELOG.md + PROTOCOL_BRIDGE.md + codebase-map.md 更新覆盖
  全部 B1-B12 变更。342/342 测试全部通过；tsc 无错；cargo check 无错。

### 新增 — Batch 1 (协议层 + app-core 基础)

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



### 修复 — 顶栏贯通 + 红绿灯避让 + drag region

- **根因**:`tauri.conf.json` 已设 `titleBarStyle: "Overlay"`(红绿灯按钮浮在
  webview 之上),但 React 树**没有任何元素为红绿灯预留避让空间**,
  `ActivityBar` 的 32px "R" logo 正好被 3 个圆点盖住。同时 `base.css` 的
  `[data-tauri-drag-region]` 规则虽然存在,**全代码库没有任何元素挂这个属性** ——
  整个窗口无法靠标题栏拖动。此外旧 `TitleBar` 只横跨主区(嵌在 `.main` 内),
  与 ActivityBar/Sidebar 视觉脱节,与「一条贯通全宽的深色顶栏」范式不符。
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

### 修复 — 主内容区永久空白（严重回归）

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

### 变更 — UI 全面重建收尾（打磨 + a11y + 测试 + 文档）

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

### 新增 — Batch 7（按工具 ToolCells + 审批历史）

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

### 新增 — Batch 11（记忆管理视图）

- **MemoryView (B11-01)**: 新 `src/features/memory/MemoryView.tsx`,
  通过 `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`
  与后端同步持久记忆。
  - Scope 过滤: All / Global / Project / Session 四个 tab,
    实时计数;`filterBar` + `addBtn` 一行完成。
  - 新增表单: scope select + key + value input + Save button,
    点击 Add 展开,Cancel 收起。
  - 内联编辑: 点击 edit → textarea + Save/Cancel; delete → Trash2。
  - 空态: "Add a key above or let the agent learn your preferences."
- **router**: 新增 `/memory` → `MemoryView` 路由。
- **测试**: MemoryView 5 个 vitest (title/filters/add toggle/form/empty state)。
  342/342 tests pass; tsc clean; cargo check clean。

### 新增 — Batch 10（CommandPalette + ⌘K 键映射 + StatusBar 会话计数）

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

### 新增 — Batch 9（文件树 + 代码编辑器 + 工作区切换器）

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

### 新增 — Batch 8（Terminal：真实 shell 执行 + 流式输出）

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

### 变更 — UI/UX 全面重建（阶段 4：剩余视图统一外壳 + stub 美化）
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

### 变更 — UI/UX 全面重建（阶段 3：高频视图精修）
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

### 变更 — UI/UX 全面重建（阶段 2：IDE 式 Shell + 核心三件套）
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

### 变更 — UI/UX 全面重建（阶段 1：设计系统地基）
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

### 新增
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

### 新增
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
- **Docs**: `docs/codebase-map.md` — 任务导向导航。
- **Docs**: `docs/PROTOCOL_BRIDGE.md` — Tauri ↔ reflect-protocol 信封规范。

### 变更
- `ReviewDecision` TS 类型对齐 Rust `#[serde(rename_all = "snake_case")]`:
  从 `'approve'|'deny'|'abort'` 改为 `'approve'|'approve_for_session'|{deny:{reason}}`。
- `reflect_*` Op 命令返回类型从 `void` 改为 `string`(submission id,供 pairing)。
- `services/agent.ts` 改为 `stores/agentStore.ts` 的兼容 re-export 层。

### 移除
- `src/App.tsx`(死代码入口,真根是 `main.tsx` → `router.tsx`)。
- `commands/mod.rs::all_commands()`(dead code)。
- 所有 view 的硬编码 `STUB_*` 数组 + phantom `invoke()`(Files/Git/Terminal/About/Update
  曾 invoke 不存在的命令,违反 AGENTS.md 规则 5)。

### 修复
- 修复 `useAgent()` 各组件持独立 turns 副本的结构 bug(改用 Zustand 单 store)。
- 修复 `handle_event` 只处理 4/33 事件类型、丢弃工具调用/思考/审批/错误的问题。
- 修复 Files/Git/Terminal/About/Update 调用不存在的后端命令的问题。

### 安全
- 工具输出经 `Sanitizer::with_defaults()`(10 个默认密钥脱敏 pattern)。

---

## 0.1.0 — 2026-07-07（初始 MVP）

首个可对外发布的里程碑。应用可启动，三栏布局正常渲染，会话列表可用，Tauri 命令桥往返打通。

### 新增
- **M1.1 脚手架** (commit `2c73335`): Tauri 2 + React 19 + Vite + TS 工作区；`reflect-desktop` 二进制名；5 个图标；capabilities。
- **M1.2 协议桥** (M1.2): 14 个 Tauri 命令 + 1 个推送事件（`reflect_event`）；`forward_agent_events` 循环；4/4 E2E 测试。
- **M1.3 三栏布局** (M1.3): 左侧栏 + 聊天 + 右侧面板 + 底部状态栏；会话列表占位。
- **M1.4 聊天渲染** (M1.4): `MessageList`、`MessageRow`、`Composer` —— 流式 / Markdown / 工具行的占位实现。
- **M1.5 Composer + Slash 弹层** (M1.5): `/` 弹层、`slashCommands.ts` 目录。
- **M1.6 Modal 套件** (M1.6): `ModalShell` + `index.tsx`，覆盖 approval / question / plan / ask_user。
- **M1.7 状态栏 + 设置** (M1.7): 顶部 + 底部栏；`SettingsView` 骨架，含 Display/Editor/Provider 区块。
- **M1.8 打磨** (M1.8): macOS overlay 标题栏（`titleBarStyle: "Overlay"`）、`macOSPrivateApi: true`、USER_GUIDE。
- **M2.x 真实后端** (commit `65bff4b`): `MinimalAgent` 换成真实 `reflect_core::AgentThread`（M2.x = stub 模型 + `EchoTool`；无网络依赖）。会话事件经 `tokio::sync::broadcast` 广播（fan-out）。
- **4 项产品联动**: `tray.rs`、`menu.rs`、`shortcut.rs`、`dock.rs`（macOS 关闭到托盘）。
- **路由** (commit `29044a4`): TanStack Router v1 + TanStack Query v5。
- **B1..B6 功能切片** (commits `d7ddb03`, `1dee7c1`, `3a85fce`, `76d7717`): Home / Threads / Models / Settings / Files / Git / Skills / Workspaces / Plan / Prompts / Notifications / Terminal 视图全部渲染。
- **测试** (commit `2ed587c`): Vitest 配置 + 初始测试（`SettingsView`、`MessageList`、`ThreadsView`、`useSessions`、`agent`）。

### 说明
- M2.x 后端有意保持 **stub**（单个 `EchoTool`、空 `ModelRegistry`）。网络 LLM 客户端和其余 21 个内置工具在 M3.x 落地。
- `reflect_delete_session` 返回 `delete not implemented in M1; use archive in M2.4`，以防误删数据。
- 所有事件在线上使用 `snake_case` 判别符（`#[serde(tag = "type", rename_all = "snake_case")]`）；Rust 的 `PascalCase` 结构体 variant 是内部细节。

---

## 版本策略

- **Major**: 协议级破坏性变更（`Op` / `EventMsg` 形状变更）。
- **Minor**: 新功能切片、新 Tauri 命令、新事件 variant（始终**增量**）。
- **Patch**: bug 修复、文档更新、内部重构。

GUI 跟随 Reflect-Agent 的主版本节奏；minor 版本可独立发布（协议是增量稳定的）。