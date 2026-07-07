/**
 * M1.3 Sidebar —— 时间分桶 session 列表。 参照 CodexMonitor
 * `src/features/threads/components/Sidebar.tsx:48-97` 简化版。
 */
import type { ReflectSessionInfo } from '@/utils/tauri';
import type { SessionBucket } from '../hooks/useSessions';

interface Props {
  buckets: SessionBucket[];
  loading: boolean;
  error: string | null;
  activeId: string | null;
  onSelect: (id: string) => void;
  onRefresh: () => void;
}

export function Sidebar({ buckets, loading, error, activeId, onSelect, onRefresh }: Props) {
  return (
    <aside
      style={{
        width: 260,
        minWidth: 200,
        maxWidth: 480,
        padding: 12,
        overflowY: 'auto',
        borderRight: '1px solid #ddd',
        background: '#fafafa',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 8 }}>
        <h3 style={{ margin: 0, flex: 1 }}>Sessions</h3>
        <button onClick={onRefresh} disabled={loading} title="Refresh">
          ↻
        </button>
      </div>
      {error && <p style={{ color: 'crimson', fontSize: 12 }}>{error}</p>}
      {loading && buckets.length === 0 && <p>Loading…</p>}
      {!loading && buckets.length === 0 && (
        <p style={{ color: '#888', fontSize: 12 }}>
          No sessions yet. Start a turn above to create one.
        </p>
      )}
      {buckets.map((b) => (
        <section key={b.label} style={{ marginBottom: 12 }}>
          <h4
            style={{
              margin: '4px 0',
              fontSize: 11,
              textTransform: 'uppercase',
              color: '#666',
            }}
          >
            {b.label}
          </h4>
          {b.sessions.map((s) => (
            <SessionItem
              key={s.session_id}
              session={s}
              active={activeId === s.session_id}
              onClick={() => onSelect(s.session_id)}
            />
          ))}
        </section>
      ))}
    </aside>
  );
}

function SessionItem({
  session,
  active,
  onClick,
}: {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
}) {
  const title = session.display_name || session.session_id.slice(0, 8);
  return (
    <button
      onClick={onClick}
      style={{
        display: 'block',
        width: '100%',
        textAlign: 'left',
        padding: '6px 8px',
        marginBottom: 4,
        border: active ? '1px solid #3b82f6' : '1px solid #eee',
        borderRadius: 4,
        background: active ? '#eef2ff' : 'white',
        cursor: 'pointer',
      }}
    >
      <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
      <div style={{ fontSize: 11, color: '#666' }}>
        {session.message_count} msgs · {session.token_total} tok
      </div>
    </button>
  );
}
