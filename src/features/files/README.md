# files (placeholder)

对标 CodexMonitor 的 `src/features/files/`。 当前（M1.x）仅有空目录；待 B1-B10 阶段逐步填实 components + hooks + stores。

参考实现：
- 文件结构：CodexMonitor-main/src/features/files/
- 镜像任务规格：docs/ARCHITECTURE.md（待 M2 更新）

## 计划接入的组件
- components/ — 7-12 个 React 组件
- hooks/    — `use<Feature>State` + `use<Feature>Actions`
- stores/   — Zustand slice (when state is local to this feature)
- services/ — IPC `@tauri-apps` invoke/listen 包装（仅 settings / workspaces）
