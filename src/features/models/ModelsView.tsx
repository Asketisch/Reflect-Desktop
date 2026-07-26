/**
 * Models —— 当前 model 卡片 + reasoning effort 分段控件（CSS Modules 版）。
 *
 * 数据源:reflect_agent_status + reflect_set_effort。
 */
import { useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { Cpu, Folder, AlertTriangle, CheckCircle2, Zap } from 'lucide-react';
import { reflect_agent_status, reflect_set_effort } from '@/utils/commands';
import { Card, Badge, Button, Icon, SegmentedControl, EmptyState } from '@/features/design-system';
import { useI18n, type LocaleKey } from '@/utils/i18n';
import s from './ModelsView.module.css';

const REASONING_EFFORTS = ['low', 'medium', 'high'] as const;
type Effort = (typeof REASONING_EFFORTS)[number];

const EFFORT_HINTS: Record<Effort, LocaleKey> = {
  low: 'models.effort.lowDesc',
  medium: 'models.effort.mediumDesc',
  high: 'models.effort.highDesc',
};

export function ModelsView() {
  const { t } = useI18n();
  const [effort, setEffort] = useState<Effort>('medium');
  const [saved, setSaved] = useState(false);

  const statusQuery = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  const effortMutation = useMutation({
    mutationFn: async (level: Effort) => reflect_set_effort(level),
    onSuccess: () => {
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    },
  });

  const status = statusQuery.data;
  const hasModel = Boolean(status?.has_model);

  return (
    <div className={s.root}>
      <header className={s.header}>
        <h1 className={s.title}>{t('models.title')}</h1>
        <p className={s.subtitle}>{t('models.subtitle')}</p>
      </header>

      {/* Current model */}
      <section>
        <h2 className={s.sectionTitle}>{t('models.currentModel')}</h2>
        {statusQuery.isLoading ? (
          <Card level="outlined" padding="md">
            <p className={s.loading}>{t('common.loading')}</p>
          </Card>
        ) : hasModel ? (
          <Card level="outlined" padding="lg" className={s.modelCard}>
            <div className={s.modelIcon}>
              <Icon icon={Cpu} size={24} />
            </div>
            <div className={s.modelBody}>
              <div className={s.modelNameRow}>
                <code className={s.modelName}>{status?.model}</code>
                <Badge variant="success" dot>{t('about.ready')}</Badge>
              </div>
              {status?.workspace && (
                <div className={s.workspaceRow}>
                  <Icon icon={Folder} size={12} />
                  <code className={s.workspace}>{status.workspace}</code>
                </div>
              )}
            </div>
          </Card>
        ) : (
          <Card level="outlined" padding="lg">
            <EmptyState
              icon={<Icon icon={AlertTriangle} />}
              title={t('models.noProvider')}
              description={
                <>
                  {status?.degraded_reason ?? t('models.noProviderDesc')} {t('models.configureHint')}
                </>
              }
              action={<Button variant="primary" size="sm" onClick={() => window.location.assign('#/settings')}>{t('palette.item.goSettings')}</Button>}
            />
          </Card>
        )}
      </section>

      {/* Reasoning effort */}
      <section>
        <h2 className={s.sectionTitle}>{t('models.effort')}</h2>
        <Card level="outlined" padding="lg">
          <div className={s.effortHeader}>
            <div className={s.effortLabel}>
              <Icon icon={Zap} size={14} />
              <span>{t('models.effortHint')}</span>
            </div>
            <SegmentedControl<Effort>
              value={effort}
              onChange={(v) => setEffort(v)}
              options={REASONING_EFFORTS.map((e) => ({ value: e, label: e, hint: t(EFFORT_HINTS[e]) }))}
              stacked
            />
          </div>
          <div className={s.effortApply}>
            <Button
              variant="primary"
              onClick={() => effortMutation.mutate(effort)}
              loading={effortMutation.isPending}
              leftIcon={<Icon icon={CheckCircle2} size={14} />}
            >
              {saved ? t('models.applied') : t('models.applyEffort')}
            </Button>
            <span className={s.effortHint}>{t(EFFORT_HINTS[effort])}</span>
          </div>
          <p className={s.note}>
            {t('models.settingsHint', { path: t('settings.provider') })}
          </p>
        </Card>
      </section>
    </div>
  );
}
