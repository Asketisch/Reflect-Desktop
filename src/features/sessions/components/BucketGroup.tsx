/**
 * BucketGroup —— 单个时间分桶（CSS Modules 版）。
 */
import type { SessionBucket } from '../utils/buckets';
import { SessionItem } from './SessionItem';
import { useI18n, type LocaleKey } from '@/utils/i18n';
import s from './BucketGroup.module.css';

export interface BucketGroupProps {
  bucket: SessionBucket;
  activeId: string | null;
  onSelect: (id: string) => void;
}

/** 内部 label → i18n key。 */
const BUCKET_LABEL_KEYS: Record<string, LocaleKey> = {
  'Now': 'sidebar.bucket.now',
  'Today': 'sidebar.bucket.today',
  'Yesterday': 'sidebar.bucket.yesterday',
  'This week': 'sidebar.bucket.thisWeek',
  'Older': 'sidebar.bucket.older',
};

export function BucketGroup({ bucket, activeId, onSelect }: BucketGroupProps) {
  const { t } = useI18n();
  const labelKey = BUCKET_LABEL_KEYS[bucket.label];
  return (
    <section className={s.group}>
      <h4 className={s.label}>{labelKey ? t(labelKey) : bucket.label}</h4>
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
