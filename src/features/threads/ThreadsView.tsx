/**
 * M3.x Threads —— 线程列表顶层视图。
 *
 * CodexMonitor 参考: `src/features/threads/components/ThreadsView.tsx`
 */
import { useSessions, useActiveSession } from '../sessions/hooks/useSessions';
import { ThreadBucketGroup } from './components/ThreadBucketGroup';

export function ThreadsView() {
  const { buckets } = useSessions();
  const { activeId, setActiveId } = useActiveSession();

  return (
    <div style={{ padding: 24, maxWidth: 600, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Threads</h1>
      {buckets.length === 0 && (
        <p style={{ color: '#888' }}>No threads yet. Start a conversation in Chat.</p>
      )}
      {buckets.map((b) => (
        <ThreadBucketGroup
          key={b.label}
          bucket={b}
          activeId={activeId}
          onSelect={setActiveId}
        />
      ))}
    </div>
  );
}