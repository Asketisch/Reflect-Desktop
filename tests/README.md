# tests/ — 应用级测试套件

本目录是 ReflectDesktop 的应用级（非冒烟）测试套件：通过一个实现**全部 IPC 命令**的
状态化模拟后端，驱动**真实前端**（React 应用完整挂载、真实 reducer、真实 React Query、
真实路由）走完完整功能路径，并对线协议（wire shape）、UI 行为与失败韧性做断言。

当前规模：10 个测试文件 / 168 个测试（另有 `src/` 内 67 个组件/模块测试文件，合计 77 文件 732 测试全绿）。

## 运行

```bash
# 全量（含 src/ 与 tests/）
pnpm test

# 仅本目录
pnpm vitest run tests/

# 单个文件（定向迭代）
pnpm vitest run tests/chat.full-flow.test.tsx
```

Vitest 收集范围已在 `vitest.config.ts` 中扩展为 `src/**` + `tests/**`，
环境（jsdom、`src/test/setup.tsx` 的 Tauri mock）与 `src/` 测试完全复用。

## 目录结构

```
tests/
├── helpers/
│   ├── fakeBackend.ts   # 全命令模拟后端 + ScriptedAgent 事件编排
│   └── appHarness.tsx   # 真实应用挂载（renderApp / renderView）
├── ipc.command-contract.test.ts               # IPC 契约与四方对齐
├── agent.protocol-events.test.ts              # 协议事件全矩阵
├── chat.full-flow.test.tsx                    # 聊天端到端主流程
├── flows.chat-plan-goal-edit.test.tsx         # chat→plan→edit 全链路 + goal 多轮续作
├── modals.approvals-questions.test.tsx        # 审批/提问/计划模态
├── sessions.lifecycle.test.tsx                # 会话列表/分桶/重放/切换
├── composer.slash-history-attachments.test.tsx# 输入框：斜杠命令/历史/附件
├── settings.config.test.tsx                   # 设置页配置读写
├── views.route-matrix.test.tsx                # 34 条路由的视图矩阵
└── errors.resilience.test.tsx                 # 错误与韧性
```

## 事件链路（被测路径 = 生产路径）

```
fakeBackend（ScriptedAgent 按脚本 emit 事件）
  → vi.mock 的 @tauri-apps/api/event listen('reflect_event')
    → agentEventBus（引用计数扇出）
      → agentStore.subscribe()（AppProviders 中建立，与生产一致）
        → reduceEvent → React 视图
```

`reflect_event` 信封保持线上格式 `{ id: string; msg: ReflectEventMsg }`，
`id === ''`（`EVENT_ID_NONE`）表示生命周期事件。用户输入经真实 Composer
（textarea + Enter）进入 `useComposerSubmission` → `reflect_submit`，
由 fakeBackend 捕获 `Submission` 信封并触发脚本化回复。

## 基础设施

### `helpers/fakeBackend.ts` — `installFakeBackend()`

- 实现 `src-tauri/src/lib.rs` `generate_handler!` 中注册的**全部** `reflect_*` 命令
  及 `ping`，返回带状态的单例句柄 `FakeBackend`：
  - `calls` / `callsOf(cmd)` / `lastArgsOf(cmd)` — 调用日志，用于断言精确的线格式
    参数（如 `reflect_rewind { toTurnId }`、`reflect_rename_session { id, newName }`）。
  - `state.*` — 可读写的内存后端状态（sessions、rollouts、config、tasks、git 等），
    测试可直接改写后触发 refetch 观察 UI 变化；所有派生返回值经 `wire()`（JSON
    round-trip）返回，模拟真实 IPC 序列化，避免 React Query 结构共享跳过更新。
  - `state.failures[cmd] = Error` — 指定命令抛错，模拟后端故障。
  - `agent.*` — 交互回执捕获：`submissions`、`toolApprovals`、`hookApprovals`、
    `planChoices`、`questionResponses`、`inputResponses`。
  - `override(cmd, handler)` — 覆盖单条命令返回值。
  - `registeredCommands` — 完整性校验用（见契约测试）。
- **ScriptedAgent**：`backend.agent.script: ScriptStep[]` 驱动 `reflect_submit`
  的回复编排：
  - `{ emit: msg | msg[] }` — 发出一个或多个 `ReflectEventMsg`；
  - `{ gate: 'tool_approval' | 'hook_approval' | 'plan_approval' | 'question' | 'input' }`
    — 挂起，轮询等待对应回执后才继续，复刻真实 agent 的审批停顿。
  - 新提交会抢占旧脚本（`turnToken`），与真实中断语义一致。
  - **脚本只绑定 `reflect_submit`**（用户文本提交）。Op 驱动的流
    （`/plan` → `reflect_enter_plan_mode`、`/goal` →
    `reflect_enter_goal_mode` 等）不经过 submit，测试中以 `emitEvent(
    返回的 submission id, msg)` 手动回放 agent 侧事件——与真实语义一致
    （Op 命令返回 submission id，agent 在同一 id 下继续产出事件）。
- goal 模式状态（`state.goal`）镜像 core 侧 `cfg.goal` 的 GoalController
  生命周期：enter 写入目标/校验命令/预算，exit 清空。
- 会话/时间/任务等 fixture 覆盖全部时间分桶（Now/今天/昨天/本周/更早）与典型字段。

### `helpers/appHarness.tsx`

- `renderApp()` — 挂载**完整应用**（`RouterProvider` + I18nProvider + 每挂载一个全新
  QueryClient + `agentStore.subscribe()`），返回 `{ navigate, pathname }`。
  不复用 `router.tsx` 里的模块级单例 QueryClient，保证测试间隔离。
- `renderView(ui)` — 仅挂载 provider 包裹的片段（如 `<ModalStack/>`）。
- `resetAppAfterEach()` — `afterEach` 中重置 store 并回到 `/`。

## 覆盖映射

| 文件 | 测试数 | 覆盖内容 |
| --- | --- | --- |
| `ipc.command-contract.test.ts` | 29 | 四方对齐：后端 `pub async fn reflect_*` ↔ 前端 `src/utils/commands/` 包装 ↔ `lib.rs` 注册表 ↔ fakeBackend 实现；逐域线格式参数往返（审批三种决定形态、会话改名、数字任务 id、计划、goal 模式可选字段两形态、定时、边车、工作区、白名单等）；失败传播。`PROTOCOL_BRIDGE.md` 覆盖率断言（≥0.85）。 |
| `agent.protocol-events.test.ts` | 38 | 全部 `EventMsg` 变体经真实 bus → store 的完整矩阵：生命周期、turn、delta、工具调用对、审批（含 plan 绕过）、协作/MCP/LSP、计划事件、配额耗尽 toast（含数字与时间窗）、未知类型运行安全。 |
| `chat.full-flow.test.tsx` | 4 | 真实挂载 + 键入发送：thinking → delta → 工具调用 → 审批门 → 完成；提交信封、转录文本、模态标题、审批回执、token 统计、完成标志；中断；旧脚本抢占；turn 级错误项。 |
| `flows.chat-plan-goal-edit.test.tsx` | 8 | **旗舰回路**：chat → plan 草稿多次更新 → `plan_ready` → 用户三选择（auto_mode / manual_approve / revise，含事件驱动关闭）→ 编辑工具（`write_file` 逐工具审批）→ 完成，断言完整 IPC 时序；`/plan` 斜杠 Op 入口 + `/exit-plan`；**goal 模式**：`/goal <text>` 线格式、agent 零提交自动多轮续作（多 turn 累积/done/转录）、`/goal clear` 退出、裸 `/goal` reject 引导。 |
| `modals.approvals-questions.test.tsx` | 14 | 三个模态全路径：工具/hook 审批的全部决定形态、多模态排队、问题单选/多选索引组装、追问文本输入与关闭语义、计划审批三选项 + 事件驱动关闭、四模态共存。 |
| `sessions.lifecycle.test.tsx` | 11 | 五桶时间分组与 i18n 标签、空态、失败态与 Refresh 恢复、重放水合（turns/工具项/loadedSessionId）、重放失败重试、会话切换再水合、`/chat` 清空、侧栏点击导航与 aria-pressed、`/rename` 斜杠改名 + 失效重取、首条消息失效会话查询。 |
| `composer.slash-history-attachments.test.tsx` | 14 | `/compact` `/plan` `/effort` `/mode` `/interrupt` 全命令、未知命令与无效值拒绝、`/model` 引导 toast、斜杠弹层（listbox + Escape）、↑/↓ 历史召回与编辑重置、文本+图片合并信封、Shift+Enter 换行、空输入不提交、提交后清空。 |
| `settings.config.test.tsx` | 8 | ready/degraded 状态徽标、配置加载失败时壳层保持可用、权限快捷按钮（4 常用 + 7 模式全量）、raw TOML 编辑保存（精确 `{toml}` 参数 + Saved 反馈）、保存失败无反馈、语言切换（`document.documentElement.lang`）。 |
| `views.route-matrix.test.tsx` | 35 | 34 条路由逐一：主内容非空、触发对应领域命令、fixture 文本可见；外加路由树完整性对照 `router.routeTree`。 |
| `errors.resilience.test.tsx` | 7 | `stream_error` 追加错误项且 turn 仍完成、会话级错误只置 lastError、配额 toast、会话列表失败 + Refresh 恢复、`reflect_submit` 失败 toast、审批 IPC 失败模态仍乐观清除（不卡死 UI）、中断后新 turn。 |

## 断言约定

- i18n 默认 locale 为 `en`；按**渲染后的英文文案**断言（如分桶标签 `This Week`/`Older`）。
- 视图组件使用 CSS Modules，类名会被 mangle —— 一律按 role / 文案 / aria-label 定位，
  不依赖类名。
- 计划就绪模态的关闭是**事件驱动**的：`approvePlan` 发 IPC 后需等
  `plan_approved` / `plan_rejected` 事件才清除 `pendingPlan`。
- 模态按钮 onClick 为 fire-and-forget；测试中如需触发预期失败，直接调用
  store action 并 `.catch()`，避免 unhandled rejection。
- 折叠项（`Collapsible` 默认收起）内容不在 DOM 中 —— 相关断言走 store 状态。

## 与生产代码的关系

- 不修改任何 `src/` 生产代码来迁就测试；`src/test/setup.tsx` 的 Tauri mock 被
  fakeBackend 完整接管（`resetMockInvoke` 后重新注册）。
- 线协议以 `docs/PROTOCOL_BRIDGE.md` 为契约，契约测试对文档覆盖率设下限，
  新增后端命令时若文档未更新会失败。
