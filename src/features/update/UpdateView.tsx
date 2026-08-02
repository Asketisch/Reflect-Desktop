/**
 * Updates —— 版本信息 + 手动更新说明（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { RefreshCw, Tag, Terminal as TerminalIcon } from 'lucide-react';
import { ping } from '@/utils/commands';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import s from './UpdateView.module.css';

const UPDATE_COMMANDS = `# 从源码构建最新版
git pull
pnpm install
pnpm tauri build

# 或重装到 /usr/local/bin
bash scripts/install.sh`;

export function UpdateView() {
  const { t } = useI18n();
  const pingQ = useQuery({ queryKey: ['ping'], queryFn: ping, staleTime: Infinity });
  const version = pingQ.data?.version ?? '—';

  return (
    <PageShell icon={RefreshCw} title={t('update.title')} width="md">
      <Card level="outlined" padding="lg" className={s.versionCard}>
        <div className={s.versionIcon}>
          <Icon icon={Tag} size={20} />
        </div>
        <div className={s.versionBody}>
          <div className={s.versionLabel}>{t('update.currentVersion')}</div>
          <code className={s.versionValue}>{version}</code>
        </div>
        <Badge variant="info">{t('update.manual')}</Badge>
      </Card>

      <Card level="outlined" padding="md" className={s.howCard}>
        <div className={s.howTitle}>
          <Icon icon={TerminalIcon} size={14} />
          <span>{t('update.howTo')}</span>
        </div>
        <p className={s.howText}>
          {t('update.howText')}
        </p>
        <pre className={s.cmdBlock}>{UPDATE_COMMANDS}</pre>
      </Card>
    </PageShell>
  );
}