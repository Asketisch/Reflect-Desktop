/**
 * M3.x Threads —— 线程列表 + 活动线程高亮。
 *
 * 复用 useSessions hook 数据,按时间分桶展示。
 * CodexMonitor 参考: src/features/threads/components/Sidebar.tsx
 */

import { Link } from '@tanstack/react-router';
import { useSessions } from '../sessions/hooks/useSessions';

export function ThreadsView() {
  const { buckets, activeId, setActiveId } = useSessions();

  return (
    <div style={{ padding: 24, maxWidth: 600, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Threads</h1>
      {buckets.length === 0 && (
        <p style={{ color: '#888' }}>No threads yet. Start a conversation in Chat.</p>
      )}
      {buckets.map((b) => (
        <section key={b.label} style={{ marginBottom: 20 }}>
          <h3 style={{ fontSize: 12, textTransform: 'uppercase', color: '#666', marginBottom: 8 }}>
            {b.label}
          </h3>
          <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
            {b.sessions.map((s) => {
              const isActive = activeId === s.session_id;
              return (
                <li key={s.session_id} style={{ marginBottom: 4 }}>
                  <Link
                    to="/chat/$sessionId"
                    params={{ sessionId: s.session_id }}
                    onClick={() => setActiveId(s.session_id)}
                    style={{
                      display: 'block',
                      padding: '10px 12px',
                      background: isActive ? '#dbeafe' : 'transparent',
                      border: '1px solid',
                      borderColor: isActive ? '#93c5fd' : '#e2e8f0',
                      borderRadius: 6,
                      textDecoration: 'none',
                      color: '#1e293b',
                      fontSize: 14,
                    }}
                  >
                    <div style={{ fontWeight: isActive ? 600 : 400 }}>
                      {s.display_name || s.session_id}
                    </div>
                    <div style={{ fontSize: 11, color: '#888', marginTop: 2 }}>
                      {new Date(s.started_at).toLocaleString()} · {s.message_count} msgs
                    </div>
                  </Link>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}
