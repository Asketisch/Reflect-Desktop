/**
 * Threads —— 线程列表（CSS Modules 版）。
 *
 * B3-03: ThreadsView 注入 rename / delete / export handlers 到 ThreadBucketGroup。
 *        所有 mutation 通过 useSessions → invalidate → 自动重刷。
 */
import { useSessions, useActiveSession } from '../sessions/hooks/useSessions';
import { ThreadBucketGroup } from './components/ThreadBucketGroup';
import { PageShell } from '@/features/shell/PageShell';
import { Card, EmptyState, Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { MessagesSquare } from 'lucide-react';
import s from './ThreadsView.module.css';

export function ThreadsView() {
  const { buckets, rename, remove, export: exportSession } = useSessions();
  const { activeId, setActiveId } = useActiveSession();
  const { t } = useI18n();

  return (
    <PageShell icon={MessagesSquare} title={t('threads.title')} width="sm">
      {buckets.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={MessagesSquare} />}
            title={t('threads.empty')}
            description={t('home.noRecent')}
          />
        </Card>
      ) : (
        <div className={s.list}>
          {buckets.map((b) => (
            <ThreadBucketGroup
              key={b.label}
              bucket={b}
              activeId={activeId}
              onSelect={setActiveId}
              onRename={rename}
              onDelete={remove}
              onExport={exportSession}
            />
          ))}
        </div>
      )}
    </PageShell>
  );
}
