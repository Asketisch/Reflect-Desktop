/**
 * AppShell —— IDE 式三栏布局。
 *
 * 阶段 A2 修复：
 *   - Sidebar onSelect 同步 URL（navigate 到 /chat/$sessionId）+ 本地高亮。
 *   - onNewChat 导航到 /chat（新对话视图）。
 */
import { useState, type ReactNode } from 'react';
import { useRouter } from '@tanstack/react-router';
import { ActivityBar } from './ActivityBar';
import { TitleBar } from './TitleBar';
import { StatusBar } from './StatusBar';
import { Inspector } from './Inspector';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { ModalStack } from '@/features/modals';
import s from './AppShell.module.css';

export function AppShell({ children }: { children?: ReactNode }) {
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
          <div className={s.content}>{children}</div>
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
