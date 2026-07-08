/**
 * ThreadBucketGroup —— 单个时间分桶标题 + thread 列表。
 */
import type { SessionBucket } from '@/features/sessions';
import { ThreadItem } from './ThreadItem';

export interface ThreadBucketGroupProps {
  bucket: SessionBucket;
  activeId: string | null;
  onSelect: (id: string) => void;
}

export function ThreadBucketGroup({ bucket, activeId, onSelect }: ThreadBucketGroupProps) {
  return (
    <section style={{ marginBottom: 20 }} data-bucket={bucket.label}>
      <h3 style={{ fontSize: 12, textTransform: 'uppercase', color: '#666', marginBottom: 8 }}>
        {bucket.label}
      </h3>
      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {bucket.sessions.map((s) => (
          <ThreadItem
            key={s.session_id}
            session={s}
            active={activeId === s.session_id}
            onClick={() => onSelect(s.session_id)}
          />
        ))}
      </ul>
    </section>
  );
}