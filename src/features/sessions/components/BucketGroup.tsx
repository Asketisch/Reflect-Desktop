/**
 * BucketGroup —— 单个时间分桶(标题 + session 列表)。
 */
import type { SessionBucket } from '../utils/buckets';
import { SessionItem } from './SessionItem';

export interface BucketGroupProps {
  bucket: SessionBucket;
  activeId: string | null;
  onSelect: (id: string) => void;
}

export function BucketGroup({ bucket, activeId, onSelect }: BucketGroupProps) {
  return (
    <section key={bucket.label} style={{ marginBottom: 12 }}>
      <h4
        style={{
          margin: '4px 0',
          fontSize: 11,
          textTransform: 'uppercase',
          color: '#666',
        }}
      >
        {bucket.label}
      </h4>
      {bucket.sessions.map((s) => (
        <SessionItem
          key={s.session_id}
          session={s}
          active={activeId === s.session_id}
          onClick={() => onSelect(s.session_id)}
        />
      ))}
    </section>
  );
}