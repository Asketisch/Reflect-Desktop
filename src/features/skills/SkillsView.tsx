/**
 * Skills & Tools —— 列出 agent 当前已注册的工具（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { Wrench, Wrench as ToolIcon } from 'lucide-react';
import { reflect_list_tools } from '@/utils/commands';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, EmptyState, Spinner } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import s from './SkillsView.module.css';

export function SkillsView() {
  const { t } = useI18n();
  const toolsQ = useQuery({ queryKey: ['tools'], queryFn: reflect_list_tools, staleTime: 30_000 });

  const tools = toolsQ.data ?? [];
  const builtin = tools.filter((t) => !t.name.startsWith('mcp__'));
  const mcp = tools.filter((t) => t.name.startsWith('mcp__'));

  return (
    <PageShell
      icon={Wrench}
      title={t('skills.title')}
      subtitle={
        <>
          {t('skills.toolsCount', { count: tools.length })} <code className={s.codeInline}>mcp__</code>{t('skills.toolsCountSuffix')}
        </>
      }
      width="md"
    >
      {toolsQ.isLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : toolsQ.error ? (
        <Card level="flat" padding="none">
          <EmptyState icon={<Icon icon={Wrench} />} title={t('skills.error')} description={t('skills.errorDesc')} />
        </Card>
      ) : (
        <>
          <ToolGroup title={t('skills.builtin')} tools={builtin} />
          {mcp.length > 0 && <ToolGroup title={t('skills.mcp')} tools={mcp} variant="accent" />}
        </>
      )}
    </PageShell>
  );
}

function ToolGroup({
  title,
  tools,
  variant = 'neutral',
}: {
  title: string;
  tools: { name: string; description: string }[];
  variant?: 'neutral' | 'accent';
}) {
  const { t } = useI18n();
  return (
    <section className={s.group}>
      <div className={s.groupHeader}>
        <h3 className={s.groupTitle}>{title}</h3>
        <Badge variant={variant}>{tools.length}</Badge>
      </div>
      {tools.length === 0 ? (
        <p className={s.empty}>{t('skills.empty')}</p>
      ) : (
        <div className={s.toolList}>
          {tools.map((t) => (
            <div key={t.name} className={s.toolRow}>
              <div className={s.toolIcon}>
                <Icon icon={ToolIcon} size={14} />
              </div>
              <div className={s.toolBody}>
                <code className={s.toolName}>{t.name}</code>
                <p className={s.toolDesc}>{t.description}</p>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
