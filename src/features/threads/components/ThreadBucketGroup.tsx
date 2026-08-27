/**
 * ThreadBucketGroup —— 单个时间分桶（CSS Modules 版）。
 *
 * B3-03: 透传 rename / delete / export / archive handlers 到 ThreadItem。
 */
import type { SessionBucket } from '@/features/sessions';
import { ThreadItem } from './ThreadItem';
import s from './ThreadBucketGroup.module.css';

export interface ThreadBucketGroupProps {
  bucket: SessionBucket;
  activeId: string | null;
  onSelect: (id: string) => void;
  onRename: (id: string, newName: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onExport: (id: string) => Promise<string | null>;
  onArchive?: (id: string) => Promise<void>;
}

export function ThreadBucketGroup({
  bucket,
  activeId,
  onSelect,
  onRename,
  onDelete,
  onExport,
  onArchive,
}: ThreadBucketGroupProps) {
  return (
    <section className={s.group} data-bucket={bucket.label}>
      <h3 className={s.label}>{bucket.label}</h3>
      <ul className={s.items}>
        {bucket.sessions.map((sess) => (
          <ThreadItem
            key={sess.session_id}
            session={sess}
            active={activeId === sess.session_id}
            onClick={() => onSelect(sess.session_id)}
            onRename={onRename}
            onDelete={onDelete}
            onExport={onExport}
            onArchive={onArchive}
          />
        ))}
      </ul>
    </section>
  );
}
