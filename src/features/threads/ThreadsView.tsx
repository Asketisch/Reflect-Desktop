/**
 * Threads —— 线程列表（CSS Modules 版）。
 */
import { useSessions, useActiveSession } from '../sessions/hooks/useSessions';
import { ThreadBucketGroup } from './components/ThreadBucketGroup';
import { PageShell } from '@/features/shell/PageShell';
import { Card, EmptyState, Icon } from '@/features/design-system';
import { MessagesSquare } from 'lucide-react';
import s from './ThreadsView.module.css';

export function ThreadsView() {
  const { buckets } = useSessions();
  const { activeId, setActiveId } = useActiveSession();

  return (
    <PageShell icon={MessagesSquare} title="Threads" width="sm">
      {buckets.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={MessagesSquare} />}
            title="No threads yet"
            description="Start a conversation in Chat to create your first thread."
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
            />
          ))}
        </div>
      )}
    </PageShell>
  );
}
