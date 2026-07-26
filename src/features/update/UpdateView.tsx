/**
 * Updates —— 版本信息 + 手动更新说明（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { RefreshCw, Tag, Terminal as TerminalIcon } from 'lucide-react';
import { ping } from '@/utils/commands';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon } from '@/features/design-system';
import s from './UpdateView.module.css';

const UPDATE_COMMANDS = `# 从源码构建最新版
git pull
pnpm install
pnpm tauri build

# 或重装到 /usr/local/bin
bash scripts/install.sh`;

export function UpdateView() {
  const pingQ = useQuery({ queryKey: ['ping'], queryFn: ping, staleTime: Infinity });
  const version = pingQ.data?.version ?? '—';

  return (
    <PageShell icon={RefreshCw} title="Updates" width="md">
      <Card level="outlined" padding="lg" className={s.versionCard}>
        <div className={s.versionIcon}>
          <Icon icon={Tag} size={20} />
        </div>
        <div className={s.versionBody}>
          <div className={s.versionLabel}>Current version</div>
          <code className={s.versionValue}>{version}</code>
        </div>
        <Badge variant="info">manual</Badge>
      </Card>

      <Card level="outlined" padding="md" className={s.howCard}>
        <div className={s.howTitle}>
          <Icon icon={TerminalIcon} size={14} />
          <span>How to update</span>
        </div>
        <p className={s.howText}>
          Automatic updates via <code className={s.codeInline}>tauri-plugin-updater</code> are not
          integrated yet. Update manually:
        </p>
        <pre className={s.cmdBlock}>{UPDATE_COMMANDS}</pre>
      </Card>
    </PageShell>
  );
}
