/**
 * PlanReadyModal —— 计划展示 + 批准/拒绝弹窗（聚焦版）。
 *
 * 行为契约(从原 modals/index.tsx 抽出,不可变):
 *   - title 为 i18n `modal.planReady.title`。
 *   - 显示 plan 文本(summary 优先,否则 plan,再否则 raw JSON)。
 *   - tertiary Reject / primary Approve(autoFocus)。
 *   - onClose 走 store `approve('plan', id, { deny: { reason: 'plan dismissed' } })`。
 */
import { useAgentStore, type PendingPlan } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import { ModalShell } from './ModalShell';
import s from './index.module.css';

export function PlanReadyModal({ plan }: { plan: PendingPlan }) {
  const { t } = useI18n();
  const approve = useAgentStore((st) => st.approve);
  const payload = plan.payload as { plan?: string; summary?: string; steps?: unknown[] };
  const planText = payload?.summary ?? payload?.plan ?? JSON.stringify(plan.payload, null, 2);
  return (
    <ModalShell
      title={t('modal.planReady.title')}
      open={true}
      onClose={() => approve('plan', plan.id, { deny: { reason: 'plan dismissed' } })}
      tertiaryAction={{
        label: t('modal.planReady.reject'),
        onClick: () => approve('plan', plan.id, { deny: { reason: 'plan rejected' } }),
      }}
      primaryAction={{
        label: t('modal.planReady.approve'),
        onClick: () => approve('plan', plan.id, 'approve'),
        autoFocus: true,
      }}
    >
      <pre className={s.planBlock}>{planText}</pre>
      <p className={s.hint}>
        {t('modal.planReady.hint')}
      </p>
    </ModalShell>
  );
}