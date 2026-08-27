/**
 * Threads —— 线程列表（CSS Modules 版）。
 *
 * B3-03: ThreadsView 注入 rename / delete / export / archive handlers
 *        到 ThreadBucketGroup。所有 mutation 通过 useSessions → invalidate
 *        → 自动重刷。
 * 归档区：列出已归档会话（`~/.reflect/sessions-archive`），支持恢复 /
 *        彻底删除。
 */
import { useSessions, useActiveSession } from '../sessions/hooks/useSessions';
import { ThreadBucketGroup } from './components/ThreadBucketGroup';
import { PageShell } from '@/features/shell/PageShell';
import { Card, EmptyState, Icon, Button } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { MessagesSquare, Archive, ArchiveRestore, Trash2 } from 'lucide-react';
import { shortTimestamp } from './utils/threadLabels';
import s from './ThreadsView.module.css';

export function ThreadsView() {
  const {
    buckets,
    archived,
    rename,
    remove,
    archive,
    unarchive,
    export: exportSession,
  } = useSessions();
  const { activeId, setActiveId } = useActiveSession();
  const { t, tp } = useI18n();

  const handleArchive = async (id: string) => {
    await archive(id);
    if (id === activeId) setActiveId(null);
  };

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
              onArchive={handleArchive}
            />
          ))}
        </div>
      )}

      <section className={s.archived} data-testid="threads-archived">
        <h3 className={s.archivedTitle}>
          <Icon icon={Archive} size={14} /> {t('threads.archived')}
          {archived.length > 0 && <span className={s.archivedCount}>{archived.length}</span>}
        </h3>
        {archived.length === 0 ? (
          <p className={s.archivedEmpty}>{t('threads.archivedEmpty')}</p>
        ) : (
          <ul className={s.archivedList}>
            {archived.map((sess) => (
              <li key={sess.session_id} className={s.archivedRow}>
                <div className={s.archivedBody}>
                  <div className={s.archivedTitleText}>
                    {sess.title || sess.session_id.slice(0, 8)}
                  </div>
                  <div className={s.archivedMeta}>
                    {shortTimestamp(sess.started_at)} ·{' '}
                    {tp('threads.msgCount', sess.message_count)}
                  </div>
                </div>
                <div className={s.archivedActions}>
                  <Button
                    size="sm"
                    variant="ghost"
                    leftIcon={<Icon icon={ArchiveRestore} size={12} />}
                    onClick={() => void unarchive(sess.session_id)}
                    data-testid={`archived-restore-${sess.session_id}`}
                  >
                    {t('threads.restore')}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    leftIcon={<Icon icon={Trash2} size={12} />}
                    onClick={() => {
                      if (confirm(t('threads.deleteConfirm', { id: sess.session_id }))) {
                        void remove(sess.session_id);
                      }
                    }}
                    data-testid={`archived-delete-${sess.session_id}`}
                  >
                    {t('threads.delete')}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </PageShell>
  );
}
