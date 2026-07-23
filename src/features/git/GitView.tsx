/**
 * Git —— 通过 reflect_git_status / diff / log 直接展示 (B6)。
 *
 * 取代之前的 "ask the agent to run git" UX。
 */
import { useEffect, useState } from 'react';
import {
  GitBranch,
  GitCommitHorizontal,
  Check,
  ArrowRight,
} from 'lucide-react';
import {
  reflect_git_status,
  reflect_git_diff,
  reflect_git_log,
  type ReflectGitStatus,
  type ReflectGitLogEntry,
} from '@/utils/tauri';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Icon } from '@/features/design-system';
import { DiffViewer } from './DiffViewer';
import s from './GitView.module.css';

type Tab = 'working' | 'staged';

export function GitView() {
  const [status, setStatus] = useState<ReflectGitStatus | null>(null);
  const [diff, setDiff] = useState('');
  const [tab, setTab] = useState<Tab>('working');
  const [log, setLog] = useState<ReflectGitLogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [s_, d_, l_] = await Promise.all([
        reflect_git_status(),
        reflect_git_diff(tab === 'staged'),
        reflect_git_log(20),
      ]);
      setStatus(s_);
      setDiff(d_);
      setLog(l_);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab]);

  if (status && !status.is_repo) {
    return (
      <PageShell icon={GitBranch} title="Git" subtitle="No git repository found." width="md">
        <Card level="outlined" padding="md">
          <p className={s.note}>
            The current workspace is not a git repository. Run <code>git init</code> or open a folder
            that contains a <code>.git</code> directory.
          </p>
        </Card>
      </PageShell>
    );
  }

  return (
    <PageShell icon={GitBranch} title="Git" subtitle="Working tree, diff, and recent log." width="lg">
      {error && <Card level="outlined" padding="md"><p className={s.error}>Error: {error}</p></Card>}

      <Card level="elevated" padding="md" className={s.headerCard}>
        <div className={s.branchRow}>
          <div className={s.branch}>
            <Icon icon={GitBranch} size={16} />
            <strong data-testid="git-branch">{status?.branch ?? '…'}</strong>
            {status?.upstream && (
              <span className={s.upstream}>↻ {status.upstream}</span>
            )}
          </div>
          <div className={s.arrows}>
            <span title="ahead">↑ {status?.ahead ?? 0}</span>
            <span title="behind">↓ {status?.behind ?? 0}</span>
          </div>
        </div>
        <div className={s.tabs}>
          <button
            type="button"
            className={s.tab}
            data-active={tab === 'working'}
            onClick={() => setTab('working')}
          >
            Working ({status?.entries.filter((e) => !e.status.startsWith('A') && !e.status.startsWith('R')).length ?? 0})
          </button>
          <button
            type="button"
            className={s.tab}
            data-active={tab === 'staged'}
            onClick={() => setTab('staged')}
          >
            Staged
          </button>
        </div>
      </Card>

      <h3 className={s.sectionTitle}>Changes</h3>
      <Card level="flat" padding="none" className={s.entryList}>
        {status?.entries.length === 0 ? (
          <div className={s.emptyState}>No changes.</div>
        ) : (
          <ul>
            {status?.entries.map((e, i) => (
              <li key={i} className={s.entry}>
                <span className={s.status} data-status={e.status}>{e.status}</span>
                <span className={s.path}>
                  {e.old_path ? `${e.old_path} → ` : ''}{e.path}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <h3 className={s.sectionTitle}>Diff ({tab})</h3>
      <DiffViewer diff={diff} emptyMessage="No diff." />

      <h3 className={s.sectionTitle}>Recent commits</h3>
      <Card level="flat" padding="none" className={s.logList}>
        {log.length === 0 ? (
          <div className={s.emptyState}>No commits.</div>
        ) : (
          <ul>
            {log.map((c) => (
              <li key={c.hash} className={s.logRow}>
                <Icon icon={GitCommitHorizontal} size={14} />
                <code className={s.short}>{c.short_hash}</code>
                <span className={s.subject}>{c.subject}</span>
                <span className={s.author}>{c.author}</span>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <button
        type="button"
        onClick={() => void refresh()}
        className={s.refresh}
        disabled={loading}
        data-testid="git-refresh"
      >
        <Icon icon={loading ? Check : ArrowRight} size={14} />
        {loading ? 'Refreshing…' : 'Refresh'}
      </button>
    </PageShell>
  );
}
