/**
 * M1.3 Sidebar —— 时间分桶 session 列表容器。
 *
 * CodexMonitor 同名: `src/features/threads/components/Sidebar.tsx`。
 */
import type { SessionBucket } from '../utils/buckets';
import { BucketGroup } from './BucketGroup';

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
        <BucketGroup key={b.label} bucket={b} activeId={activeId} onSelect={onSelect} />
      ))}
    </aside>
  );
}