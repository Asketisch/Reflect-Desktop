/**
 * AppShell —— IDE 式三栏布局（贯通顶栏版）。
 *
 * 结构（从上到下）：
 *   ┌────────────────────────────────────────────────────┐
 *   │ TitleBar (贯通全宽，红绿灯 + drag region)            │  ← .shell[0]
 *   ├──────┬────────────┬──────────────┬─────────────────┤
 *   │Activ │ Sidebar    │ Main content │ Inspector       │  ← .body
 *   │ Bar  │ (sessions) │ (Outlet)     │                 │
 *   ├──────┴────────────┴──────────────┴─────────────────┤
 *   │ StatusBar                                          │  ← .shell[2]
 *   └────────────────────────────────────────────────────┘
 *
 * 关键决策：TitleBar 是 .shell 的第一个子元素（**与 ActivityBar 同级，而非嵌在 .main 内**），
 * 这样它横跨整个窗口宽度。macOS 红绿灯按钮（titleBarStyle: "Overlay"）嵌在顶栏左侧
 * （由 TitleBar 的 `--traffic-light-gutter` 左 padding 让位），与常见 IDE 顶栏范式一致。
 *
 * **TanStack Router v1 根路由契约**：rootRoute 的 component 必须渲染 `<Outlet />`
 * 才能把匹配到的子路由（HomeView / ChatView / SettingsView / …）挂到 DOM。
 *
 * 阶段 A2 历史：
 *   - Sidebar onSelect 同步 URL（navigate 到 /chat/$sessionId）+ 本地高亮。
 *   - onNewChat 导航到 /chat（新对话视图）。
 *
 * 结构化重构（A11+）：
 *   - 主题切换状态机 → `./hooks/useThemeCycle`。
 *   - 全局快捷键（⌘K / Esc）→ `./hooks/useCommandPaletteShortcut`。
 *   - 命令面板动作（newSession / clearAllSessions / exportActive / saveConfig / runSlash）
 *     → `./hooks/usePaletteActions`。
 *   - AppShell 本身只负责布局与 DOM 拼装。
 */
import { useState } from 'react';
import { useRouter, Outlet } from '@tanstack/react-router';
import { ActivityBar } from './ActivityBar';
import { TitleBar } from './TitleBar';
import { StatusBar } from './StatusBar';
import { Inspector } from './Inspector';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { ModalStack } from '@/features/modals';
import { CommandPalette } from '@/features/command-palette/CommandPalette';
import { useI18n } from '@/utils/i18n';
import { useAgentNotifications, loadNotifyOptions } from '@/utils/notify';
import { reflect_create_session } from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';
import { useThemeCycle } from './hooks/useThemeCycle';
import { useCommandPaletteShortcut } from './hooks/useCommandPaletteShortcut';
import { usePaletteActions } from './hooks/usePaletteActions';
import { useCurrentWorkspace } from './hooks/useCurrentWorkspace';
import s from './AppShell.module.css';

export function AppShell() {
  const { currentWorkspace } = useCurrentWorkspace();
  const sessions = useSessions({ workspacePath: currentWorkspace });
  const { activeId, setActiveId } = useActiveSession();
  const router = useRouter();
  const { t, tp } = useI18n();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [inspectorOpen, setInspectorOpen] = useState(false);

  // B13-B15: agent 完成时给 chime + 系统通知 + dock badge。
  useAgentNotifications(loadNotifyOptions());

  // 调色板所需的主题状态
  const { resolved, cycleTheme, setThemeMode } = useThemeCycle();

  // ⌘K / Ctrl+K 全局快捷键 + Esc 关闭
  const { paletteOpen, setPaletteOpen } = useCommandPaletteShortcut();

  const handleSelect = (id: string) => {
    setActiveId(id);
  };

  // 新建会话:后端预分配 session id → navigate 到 /chat/<id>。
  // workspace 归属不在这里落盘,由首条 submission 携带 `workspace`
  // 触发后端写 `SessionMeta`(见 PROTOCOL_BRIDGE §2)。
  const handleNewChat = () => {
    const pushToast = useAgentStore.getState().pushToast;
    void reflect_create_session()
      .then((id) => {
        setActiveId(id);
      })
      .catch((e: unknown) => {
        pushToast({
          kind: 'error',
          message: t('toast.newSessionFailed', { msg: e instanceof Error ? e.message : String(e) }),
        });
      });
  };

  // 删除 / 归档当前打开的会话后回到新对话视图，避免停留在已不存在的 session。
  const handleDeleteSession = async (id: string) => {
    await sessions.remove(id);
    if (id === activeId) setActiveId(null);
  };
  const handleArchiveSession = async (id: string) => {
    await sessions.archive(id);
    if (id === activeId) setActiveId(null);
  };

  // palette action handlers (集中到 hook)
  const { newSession, clearAllSessions, exportActive, saveConfig, runSlash } = usePaletteActions({
    activeId,
    onNewChat: handleNewChat,
    toasts: {
      newSession: t('toast.newSession'),
      noSessionsToClear: t('toast.noSessionsToClear'),
      deleteFailed: (msg) => t('toast.deleteFailed', { msg }),
      clearedSessions: (count) => tp('toast.clearedSessions', count, { count }),
      noActiveToExport: t('toast.noActiveToExport'),
      exported: (path) => t('composer.exported', { path }),
      exportedNoPath: t('composer.exportedNoPath'),
      exportFailed: (msg) => t('toast.exportFailed', { msg }),
      configReloaded: t('toast.configReloaded'),
      saveFailed: (msg) => t('toast.saveFailed', { msg }),
    },
  });

  return (
    <div className={s.shell}>
      {/* 贯通全宽顶栏：红绿灯按钮（macOS Overlay）嵌在里面。
       * 必须是 .shell 的第一个子元素，让 .body（ActivityBar + Sidebar + Main + Inspector）
       * 全部从顶栏下方开始 —— 「一条贯通全宽的深色顶栏」范式。
       */}
      <TitleBar
        sidebarOpen={sidebarOpen}
        onToggleSidebar={() => setSidebarOpen((v) => !v)}
        inspectorOpen={inspectorOpen}
        onToggleInspector={() => setInspectorOpen((v) => !v)}
      />
      <div className={s.body}>
        <ActivityBar />
        {sidebarOpen && (
          <aside className={s.sidebar} aria-label={t('sidebar.sessions')} data-testid="shell-sidebar">
              <Sidebar
              buckets={sessions.buckets}
              loading={sessions.loading}
              error={sessions.error}
              activeId={activeId}
              onSelect={handleSelect}
              onRefresh={sessions.refresh}
              onNewChat={handleNewChat}
              onRename={sessions.rename}
              onDelete={handleDeleteSession}
              onExport={sessions.export}
              onArchive={handleArchiveSession}
            />
          </aside>
        )}
        <main className={s.main}>
          <div className={s.content}>
            {/* TanStack Router v1：渲染匹配到的子路由。 */}
            <Outlet />
          </div>
        </main>
        {inspectorOpen && (
          <aside className={s.inspector} aria-label={t('shell.inspector')} data-testid="shell-inspector">
            <Inspector />
          </aside>
        )}
      </div>
      <StatusBar />
      <ModalStack />
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        navigate={(to) => void router.navigate({ to })}
        cycleTheme={cycleTheme}
        setTheme={setThemeMode}
        resolvedTheme={resolved}
        newSession={newSession}
        clearAllSessions={() => void clearAllSessions()}
        exportActive={() => void exportActive()}
        saveConfig={() => void saveConfig()}
        runSlash={runSlash}
      />
    </div>
  );
}
