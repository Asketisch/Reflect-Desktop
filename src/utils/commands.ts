/**
 * Frontend IPC wrappers for the Reflect GUI.
 *
 * Wrappers are now organised by domain under `./commands/{domain}.ts`.
 * This file is kept as a back-compat re-export so existing
 * `@/utils/commands` imports continue to work unchanged.
 *
 * **新代码请直接 import 自 `@/utils/commands` 或 `@/utils/commands/{domain}`。
 * `src/utils/tauri` 仍保留旧 barrel 作兼容入口。**
 *
 * 设计蓝图：`docs/PROTOCOL_BRIDGE.md` §2 + §6。
 */
export * from './commands/index';
