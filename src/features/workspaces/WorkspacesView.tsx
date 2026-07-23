/**
 * Workspaces —— 从真实 sessions 派生的工作区列表（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { FolderOpen, CheckCircle2 } from 'lucide-react';
import { reflect_agent_status, reflect_list_sessions } from '@/utils/tauri';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Spinner, EmptyState } from '@/features/design-system';
import { relativeTime } from '@/utils/time';
import s from './WorkspacesView.module.css';

interface WorkspaceEntry {
  path: string;
  sessionCount: number;
  lastUsed: string;
}

export function WorkspacesView() {
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });
  const sessionsQ = useQuery({
    queryKey: ['sessions'],
    queryFn: () => reflect_list_sessions(),
    staleTime: 60_000,
  });

  const workspaces: WorkspaceEntry[] = (() => {
    const map = new Map<string, WorkspaceEntry>();
    for (const sess of sessionsQ.data ?? []) {
      const cwd = sess.cwd || '(unknown)';
      const existing = map.get(cwd);
      if (existing) {
        existing.sessionCount += 1;
        if (sess.started_at > existing.lastUsed) existing.lastUsed = sess.started_at;
      } else {
        map.set(cwd, { path: cwd, sessionCount: 1, lastUsed: sess.started_at });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.lastUsed.localeCompare(a.lastUsed));
  })();

  const currentWs = statusQ.data?.workspace;

  return (
    <PageShell
      icon={FolderOpen}
      title="Workspaces"
      subtitle="Recently used project directories, aggregated from sessions."
      width="md"
    >
      <Card level="outlined" padding="md" className={s.currentCard}>
        <div className={s.currentRow}>
          <Icon icon={CheckCircle2} size={14} />
          <span className={s.currentLabel}>Active workspace</span>
          <code className={s.currentPath}>{currentWs ?? '(loading…)'}</code>
        </div>
        <p className={s.note}>
          Switching workspaces requires setting <code className={s.codeInline}>cwd</code> at startup,
          or via the agent's <code className={s.codeInline}>EnterWorktree</code> /{' '}
          <code className={s.codeInline}>ExitWorktree</code> tools (UI triggers coming later).
        </p>
      </Card>

      <h3 className={s.sectionTitle}>Recent workspaces</h3>
      {sessionsQ.isLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : workspaces.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={FolderOpen} />}
            title="No sessions recorded yet"
            description="Start a chat to record your first workspace."
          />
        </Card>
      ) : (
        <div className={s.list}>
          {workspaces.map((w) => {
            const isActive = w.path === currentWs;
            return (
              <Card key={w.path} level="outlined" padding="sm" className={s.wsCard} data-active={isActive || undefined}>
                <div className={s.wsIcon}>
                  <Icon icon={FolderOpen} size={16} />
                </div>
                <div className={s.wsBody}>
                  <div className={s.wsNameRow}>
                    <span className={s.wsName}>{basename(w.path)}</span>
                    {isActive && <Badge variant="success" dot>active</Badge>}
                  </div>
                  <div className={s.wsMeta}>
                    <code className={s.wsPath}>{w.path}</code>
                    <span className={s.wsSep}>·</span>
                    <span>{w.sessionCount} session{w.sessionCount === 1 ? '' : 's'}</span>
                    <span className={s.wsSep}>·</span>
                    <span>{relativeTime(w.lastUsed)}</span>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </PageShell>
  );
}

function basename(p: string): string {
  const parts = p.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || p;
}
