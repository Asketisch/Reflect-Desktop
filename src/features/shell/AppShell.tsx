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
import { useState } from 'react';
import { useRouter, Outlet } from '@tanstack/react-router';
import { ActivityBar } from './ActivityBar';
import { TitleBar } from './TitleBar';
import { StatusBar } from './StatusBar';
import { Inspector } from './Inspector';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { ModalStack } from '@/features/modals';
import s from './AppShell.module.css';

export function AppShell() {
  const sessions = useSessions();
  const { activeId, setActiveId } = useActiveSession();
  const router = useRouter();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [inspectorOpen, setInspectorOpen] = useState(false);

  const handleSelect = (id: string) => {
    setActiveId(id);
    // 同步 URL —— 让 ChatView 能感知 session 切换。
    void router.navigate({ to: '/chat/$sessionId', params: { sessionId: id } });
  };

  const handleNewChat = () => {
    setActiveId(null);
    void router.navigate({ to: '/chat' });
  };

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
          <aside className={s.sidebar} aria-label="Sessions" data-testid="shell-sidebar">
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
    </div>
  );
}
