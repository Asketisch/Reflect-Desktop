/**
 * Workspaces —— 从真实 sessions 派生的工作区列表 (B9-04 升级)。
 *
 * 顶部 active workspace + 切换按钮(bottom-right Cards 有 "Use" action);
 * 列表按 lastUsed 倒序;active 用 success badge 标出。
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { FolderOpen, CheckCircle2, ArrowRight } from 'lucide-react';
import {
  reflect_agent_status,
  reflect_list_sessions,
  reflect_set_workspace,
} from '@/utils/commands';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Spinner, EmptyState, Button } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { relativeTime } from '@/utils/time';
import { useAgentStore } from '@/stores/agentStore';
import s from './WorkspacesView.module.css';

interface WorkspaceEntry {
  path: string;
  sessionCount: number;
  lastUsed: string;
}

export function WorkspacesView() {
  const { t, tp } = useI18n();
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);
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

  const currentWs = statusQ.data?.workspace;

  // `ReflectSessionInfo`（后端 `SessionInfo`）仅携带 `session_id`,
  // `model`, `started_at`, `message_count` —— 没有 `cwd` 字段。在
  // 后端暴露 per-session workspace 之前，我们展示当前工作区卡片
  // 以及最近的 session 数量。
  const totalSessions = sessionsQ.data?.length ?? 0;
  const lastUsed = sessionsQ.data?.[0]?.started_at;

  const workspaces: WorkspaceEntry[] = currentWs
    ? [
        {
          path: currentWs,
          sessionCount: totalSessions,
          lastUsed: lastUsed ?? new Date().toISOString(),
        },
      ]
    : [];

  const onUse = async (path: string) => {
    try {
      await reflect_set_workspace(path);
      pushToast({ kind: 'success', message: t('workspaces.setTo', { name: basename(path) }) });
      await qc.invalidateQueries({ queryKey: ['agent-status'] });
    } catch (e) {
      pushToast({
        kind: 'error',
        message: t('workspaces.switchFailed', { msg: e instanceof Error ? e.message : String(e) }),
      });
    }
  };

  return (
    <PageShell
      icon={FolderOpen}
      title={t('workspaces.title')}
      subtitle={t('workspaces.subtitle')}
      width="md"
    >
      <Card level="outlined" padding="md" className={s.currentCard}>
        <div className={s.currentRow}>
          <Icon icon={CheckCircle2} size={14} />
          <span className={s.currentLabel}>{t('workspaces.current')}</span>
          <code className={s.currentPath} data-testid="workspaces-current">
            {currentWs ?? t('common.loading')}
          </code>
        </div>
        <p className={s.note}>
          {t('workspaces.note')}
        </p>
      </Card>

      <h3 className={s.sectionTitle}>{t('workspaces.recent')}</h3>
      {sessionsQ.isLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : workspaces.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={FolderOpen} />}
            title={t('workspaces.empty')}
            description={t('workspaces.emptyDesc')}
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="workspaces-list">
          {workspaces.map((w) => {
            const isActive = w.path === currentWs;
            return (
              <Card
                key={w.path}
                level="outlined"
                padding="sm"
                className={s.wsCard}
                data-active={isActive || undefined}
                data-testid={`workspace-card-${basename(w.path)}`}
              >
                <div className={s.wsIcon}>
                  <Icon icon={FolderOpen} size={16} />
                </div>
                <div className={s.wsBody}>
                  <div className={s.wsNameRow}>
                    <span className={s.wsName}>{basename(w.path)}</span>
                    {isActive && <Badge variant="success" dot>{t('home.active')}</Badge>}
                  </div>
                  <div className={s.wsMeta}>
                    <code className={s.wsPath}>{w.path}</code>
                    <span className={s.wsSep}>·</span>
                    <span>
                      {tp('workspaces.sessionCount', w.sessionCount, { count: w.sessionCount })}
                    </span>
                    <span className={s.wsSep}>·</span>
                    <span>{relativeTime(w.lastUsed)}</span>
                  </div>
                </div>
                {!isActive && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => onUse(w.path)}
                    data-testid={`workspace-use-${basename(w.path)}`}
                  >
                    {t('workspaces.use')} <Icon icon={ArrowRight} size={12} />
                  </Button>
                )}
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