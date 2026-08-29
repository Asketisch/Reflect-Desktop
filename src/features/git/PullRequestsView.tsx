/**
 * PullRequestsView —— 拉取请求列表（P3，对标 Codex「拉取请求」侧边栏入口）。
 *
 * 数据源：`reflect_gh_pr_list`（后端 shell out 到 `gh pr list --json`）。
 * gh 未安装 / 非 git repo / 未登录时后端返回错误原因 —— 这里空态展示原因
 * 而不是报错打断。点击 PR 在系统浏览器打开（target=_blank）。
 */
import { useEffect, useState } from 'react';
import { GitPullRequestArrow, ExternalLink } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Spinner, EmptyState, Button } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { reflect_gh_pr_list, type ReflectGhPullRequest } from '@/utils/commands';
import s from './PullRequestsView.module.css';

export function PullRequestsView() {
  const { t, tp } = useI18n();
  const [prs, setPrs] = useState<ReflectGhPullRequest[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    setError(null);
    reflect_gh_pr_list(30)
      .then((list) => {
        setPrs(list);
        setError(null);
      })
      .catch((e: unknown) => {
        setPrs([]);
        setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <PageShell icon={GitPullRequestArrow} title={t('pulls.title')} subtitle={t('pulls.subtitle')} width="lg">
      <div className={s.toolbar}>
        <Button size="sm" variant="secondary" onClick={refresh} disabled={loading} data-testid="pulls-refresh">
          {t('git.refresh')}
        </Button>
      </div>

      {loading && (
        <Card level="flat" padding="lg">
          <div className={s.center}><Spinner size={14} /> {t('common.loading')}</div>
        </Card>
      )}

      {!loading && error && (
        <Card level="flat" padding="lg">
          <EmptyState
            icon={<Icon icon={GitPullRequestArrow} size={20} />}
            title={t('pulls.unavailable')}
            description={error}
          />
        </Card>
      )}

      {!loading && !error && prs.length === 0 && (
        <Card level="flat" padding="lg">
          <EmptyState
            icon={<Icon icon={GitPullRequestArrow} size={20} />}
            title={t('pulls.empty')}
            description={t('pulls.emptyDesc')}
          />
        </Card>
      )}

      {!loading && prs.length > 0 && (
        <Card level="flat" padding="none">
          <div className={s.summary}>
            <Badge variant="info">{tp('pulls.count', prs.length, { count: prs.length })}</Badge>
          </div>
          <ul className={s.list}>
            {prs.map((pr) => (
              <li key={pr.number} className={s.row}>
                <a className={s.link} href={pr.url} target="_blank" rel="noreferrer" data-testid={`pull-${pr.number}`}>
                  <span className={s.iconWrap}><Icon icon={GitPullRequestArrow} size={14} /></span>
                  <span className={s.main}>
                    <span className={s.title}>
                      <code className={s.number}>#{pr.number}</code> {pr.title}
                      {pr.draft && <Badge variant="warning">{t('pulls.draft')}</Badge>}
                    </span>
                    <span className={s.meta}>
                      {pr.author && <span>{pr.author}</span>}
                      <span className={s.headRef}>{pr.head_ref}</span>
                    </span>
                  </span>
                  <Icon icon={ExternalLink} size={12} className={s.external} />
                </a>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </PageShell>
  );
}
