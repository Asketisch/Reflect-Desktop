/**
 * AppShell —— IDE 式三栏布局。
 *
 * **TanStack Router v1 根路由契约**：rootRoute 的 component 必须渲染 `<Outlet />`
 * 才能把匹配到的子路由（HomeView / ChatView / SettingsView / …）挂到 DOM。
 * 早期版本误用 `children` prop —— Router 不会向 root component 传 children，
 * 导致整个主内容区永久空白（标题栏/侧栏/状态栏正常，唯独内容空）。
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
      <ActivityBar />
      <div className={s.body}>
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
          <TitleBar
            sidebarOpen={sidebarOpen}
            onToggleSidebar={() => setSidebarOpen((v) => !v)}
            inspectorOpen={inspectorOpen}
            onToggleInspector={() => setInspectorOpen((v) => !v)}
          />
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
