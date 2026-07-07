/**
 * M3.x Home —— 欢迎仪表盘。
 *
 * - 最近 session 快照 (useSessions hook)
 * - 快速操作: New Chat / Open Workspace / Settings
 * - 当前 model + provider 展示
 */

import { useMemo } from 'react';
import { Link } from '@tanstack/react-router';
import { useSessions } from '@/features/sessions/hooks/useSessions';
import { useAgent } from '@/services/agent';

export function HomeView() {
  const { buckets } = useSessions();
  const { session } = useAgent();

  const recent = useMemo(() => {
    const all: Array<{ id: string; label: string; started_at: string }> = [];
    for (const b of buckets) {
      for (const s of b.sessions) {
        all.push({ id: s.session_id, label: s.display_name || s.session_id, started_at: s.started_at });
      }
    }
    all.sort((a, b) => b.started_at.localeCompare(a.started_at));
    return all.slice(0, 8);
  }, [buckets]);

  return (
    <div style={{ padding: 32, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 28, marginBottom: 4 }}>Welcome to Reflect</h1>
      <p style={{ color: '#666', marginBottom: 24 }}>
        AI coding agent — start a conversation or pick up where you left off.
      </p>

      {session && (
        <div style={{ padding: 12, background: '#f0f9ff', borderRadius: 8, marginBottom: 24, fontSize: 13 }}>
          <strong>Active:</strong> {session.model} @ {session.provider}
        </div>
      )}

      <div style={{ display: 'flex', gap: 12, marginBottom: 32 }}>
        <QuickLink to="/chat" label="New Chat" icon="+" />
        <QuickLink to="/workspaces" label="Workspaces" icon="◫" />
        <QuickLink to="/settings" label="Settings" icon="⚙" />
        <QuickLink to="/models" label="Models" icon="◎" />
      </div>

      <section>
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Recent Sessions</h2>
        {recent.length === 0 && (
          <p style={{ color: '#888', fontSize: 13 }}>No sessions yet. Start chatting to create one.</p>
        )}
        <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
          {recent.map((s) => (
            <li key={s.id} style={{ marginBottom: 6 }}>
              <Link
                to="/chat/$sessionId"
                params={{ sessionId: s.id }}
                style={{ textDecoration: 'none', color: '#3b82f6', fontSize: 14 }}
              >
                {s.label}
              </Link>
              <span style={{ color: '#aaa', fontSize: 11, marginLeft: 8 }}>
                {new Date(s.started_at).toLocaleString()}
              </span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

function QuickLink({ to, label, icon }: { to: string; label: string; icon: string }) {
  return (
    <Link
      to={to}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '10px 16px',
        background: '#f8fafc',
        border: '1px solid #e2e8f0',
        borderRadius: 8,
        textDecoration: 'none',
        color: '#1e293b',
        fontSize: 14,
        fontWeight: 500,
        transition: 'background 0.15s',
      }}
    >
      <span style={{ fontSize: 18 }}>{icon}</span>
      {label}
    </Link>
  );
}
