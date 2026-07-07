/**
 * M1.7 三栏 + 顶/底栏 + Settings 入口。
 *
 * M1.8 收尾时升级为 react-resizable-panels。
 */
import { useState } from 'react';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions } from '@/features/sessions/hooks/useSessions';
import { ChatView } from '@/features/messages/ChatView';
import { SettingsView } from '@/features/settings/SettingsView';
import { Topbar, BottomBar } from '@/widgets/StatusBar';

interface Props {
  children?: React.ReactNode;
}

export function AppLayout({ children }: Props) {
  const sessions = useSessions();
  const [showRight, setShowRight] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);

  if (settingsOpen) {
    return <SettingsView onClose={() => setSettingsOpen(false)} />;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <Topbar />
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <Sidebar
          buckets={sessions.buckets}
          loading={sessions.loading}
          error={sessions.error}
          activeId={sessions.activeId}
          onSelect={sessions.setActiveId}
          onRefresh={sessions.refresh}
        />
        <main style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <div
            style={{
              padding: '4px 16px',
              borderBottom: '1px solid #eee',
              display: 'flex',
              gap: 8,
              alignItems: 'center',
              fontSize: 12,
            }}
          >
            <div style={{ flex: 1 }} />
            <button onClick={() => setShowRight((v) => !v)}>
              {showRight ? 'Hide right panel' : 'Show right panel'}
            </button>
            <button onClick={() => setSettingsOpen(true)}>Settings</button>
          </div>
          <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
            <ChatView />
          </div>
        </main>
        {showRight && (
          <aside
            style={{
              width: 340,
              padding: 12,
              borderLeft: '1px solid #ddd',
              background: '#fafafa',
              overflowY: 'auto',
            }}
          >
            <h3 style={{ marginTop: 0 }}>Right panel</h3>
            <p style={{ color: '#888', fontSize: 12 }}>
              M2.5+ 挂 task 面板 / tool inspector / cost tracker。 M1 暂留空。
            </p>
            {children}
          </aside>
        )}
      </div>
      <BottomBar />
    </div>
  );
}
