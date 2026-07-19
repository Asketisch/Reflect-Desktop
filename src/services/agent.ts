/**
 * agent service —— 兼容 re-export 层。
 *
 * ## 历史
 *
 * M1.x 这里是 `useAgent()` hook + `handle_event` reducer(本地 useState)。
 * 阶段 3a 起,真实状态逻辑迁移到 `@/stores/agentStore`(Zustand 单 store),
 * 修复了"每个组件持独立 turns 副本"的结构 bug。
 *
 * 本文件保留为薄 re-export,让现有 import `@/services/agent` 的代码
 * (MessageList / Composer / HomeView / DebugView / StatusBar 及其测试)
 * 无需改 import 路径即可拿到新 store 的 `useAgent`。
 *
 * 新代码应直接 `import { useAgentStore } from '@/stores/agentStore'`,
 * 用细粒度 selector 避免 re-render 过宽。
 */
export { useAgent, type Turn, type TurnItem, type TurnStatus } from '@/stores/agentStore';
export { reduceEvent as handle_event } from '@/stores/agentStore';
