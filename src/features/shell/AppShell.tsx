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
 * （由 TitleBar 的 `--traffic-light-gutter` 左 padding 让位），对齐 ZCode/Codex 范式。
 *
 * **TanStack Router v1 根路由契约**：rootRoute 的 component 必须渲染 `<Outlet />`
 * 才能把匹配到的子路由（HomeView / ChatView / SettingsView / …）挂到 DOM。
 *
 * 阶段 A2 历史：
 *   - Sidebar onSelect 同步 URL（navigate 到 /chat/$sessionId）+ 本地高亮。
 *   - onNewChat 导航到 /chat（新对话视图）。
 */
import { useCallback, useEffect, useState } from 'react';
import { useRouter, Outlet } from '@tanstack/react-router';
import { ActivityBar } from './ActivityBar';
import { TitleBar } from './TitleBar';
import { StatusBar } from './StatusBar';
import { Inspector } from './Inspector';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { ModalStack } from '@/features/modals';
import { CommandPalette } from '@/features/command-palette/CommandPalette';
import { getTheme, getResolvedTheme, setTheme, subscribeTheme, type ThemeMode } from '@/utils/theme';
import { useAgentStore } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import { reflect_export_session, reflect_save_config } from '@/utils/commands';
import { useQueryClient } from '@tanstack/react-query';
import { reflect_list_sessions, reflect_delete_session } from '@/utils/commands';
import s from './AppShell.module.css';

export function AppShell() {
  const sessions = useSessions();
  const { activeId, setActiveId } = useActiveSession();
  const router = useRouter();
  const qc = useQueryClient();
  const submit = useAgentStore((st) => st.submit);
  const pushToast = useAgentStore((st) => st.pushToast);
  const { t } = useI18n();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  // theme state for palette
  const [mode, setMode] = useState<ThemeMode>(() => getTheme());
  const [resolved, setResolved] = useState<'light' | 'dark'>(() => getResolvedTheme());
  useEffect(() => subscribeTheme(setResolved), []);
  const cycleTheme = useCallback(() => {
    const next: ThemeMode = mode === 'dark' ? 'light' : mode === 'light' ? 'system' : 'dark';
    setTheme(next);
    setMode(next);
  }, [mode]);

  // ⌘K / Ctrl+K global shortcut
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPaletteOpen((v) => !v);
      } else if (e.key === 'Escape' && paletteOpen) {
        setPaletteOpen(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [paletteOpen]);

  const handleSelect = (id: string) => {
    setActiveId(id);
  };

  const handleNewChat = () => {
    setActiveId(null);
    void router.navigate({ to: '/chat' });
  };

  // palette action handlers
  const newSession = useCallback(() => {
    handleNewChat();
    pushToast({ kind: 'info', message: 'New session started.' });
  }, [pushToast]);

  const clearAllSessions = useCallback(async () => {
    const list = await qc.fetchQuery({ queryKey: ['sessions'], queryFn: () => reflect_list_sessions() });
    if (!Array.isArray(list) || list.length === 0) {
      pushToast({ kind: 'info', message: 'No sessions to clear.' });
      return;
    }
    for (const s of list) {
      try {
        await reflect_delete_session(s.session_id);
      } catch (e) {
        pushToast({ kind: 'error', message: `Delete failed: ${(e as Error).message}` });
      }
    }
    await qc.invalidateQueries({ queryKey: ['sessions'] });
    pushToast({ kind: 'success', message: `Cleared ${list.length} session(s).` });
  }, [qc, pushToast]);

  const exportActive = useCallback(async () => {
    if (!activeId) {
      pushToast({ kind: 'warn', message: 'No active session to export.' });
      return;
    }
    try {
      const path = await reflect_export_session(activeId);
      pushToast({ kind: 'success', message: `Exported → ${path ?? '(no path)'}` });
    } catch (e) {
      pushToast({ kind: 'error', message: `Export failed: ${(e as Error).message}` });
    }
  }, [activeId, pushToast]);

  const saveConfig = useCallback(async () => {
    try {
      await reflect_save_config('');
      pushToast({ kind: 'success', message: 'Config reloaded.' });
    } catch (e) {
      pushToast({ kind: 'error', message: `Save failed: ${(e as Error).message}` });
    }
  }, [pushToast]);

  const runSlash = useCallback(
    (slash: string) => {
      // palette triggers fire-and-forget via submit; this lets e.g. /compact work
      // without forcing the user to type into the composer.
      void submit(slash);
    },
    [submit],
  );

  return (
    <div className={s.shell}>
      {/* 贯通全宽顶栏：红绿灯按钮（macOS Overlay）嵌在里面。
       * 必须是 .shell 的第一个子元素，让 .body（ActivityBar + Sidebar + Main + Inspector）
       * 全部从顶栏下方开始 —— 对齐 ZCode/Codex 的「一条深色顶栏」范式。
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
          <aside className={s.inspector} aria-label="Inspector" data-testid="shell-inspector">
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
        setTheme={(m) => { setTheme(m); setMode(m); }}
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
