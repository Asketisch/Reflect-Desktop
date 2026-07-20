/**
 * BucketGroup —— 单个时间分桶（CSS Modules 版）。
 */
import type { SessionBucket } from '../utils/buckets';
import { SessionItem } from './SessionItem';
import s from './BucketGroup.module.css';

export interface BucketGroupProps {
  bucket: SessionBucket;
  activeId: string | null;
  onSelect: (id: string) => void;
}

export function BucketGroup({ bucket, activeId, onSelect }: BucketGroupProps) {
  return (
    <section className={s.group}>
      <h4 className={s.label}>{bucket.label}</h4>
      <div className={s.items}>
        {bucket.sessions.map((sess) => (
          <SessionItem
            key={sess.session_id}
            session={sess}
            active={activeId === sess.session_id}
            onClick={() => onSelect(sess.session_id)}
          />
        ))}
      </div>
    </section>
  );
}
