/**
 * ApprovalModal —— 工具 / Hook / Plan 权限审批弹窗（聚焦版）。
 *
 * 行为契约(从原 modals/index.tsx 抽出,不可变):
 *   - title 按 kind 取 tool / hook / plan 对应 i18n 标题。
 *   - 三按钮:tertiary Deny / secondary Approve-for-session / primary Approve(autoFocus)。
 *   - onClose 走 store `approve(kind, id, { deny: { reason: 'dismissed' } })`。
 *   - 内容仅渲染 toolName(若有)+ argsSummary(若有)+ hint 文案。
 */
import { useAgentStore, type PendingApproval } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import { ModalShell } from './ModalShell';
import s from './index.module.css';

export function ApprovalModal({ approval }: { approval: PendingApproval }) {
  const { t } = useI18n();
  const approve = useAgentStore((st) => st.approve);
  const kindLabel =
    approval.kind === 'tool'
      ? t('modal.approval.toolTitle')
      : approval.kind === 'hook'
        ? t('modal.approval.hookTitle')
        : t('modal.approval.planTitle');
  return (
    <ModalShell
      title={kindLabel}
      open={true}
      onClose={() => approve(approval.kind, approval.id, { deny: { reason: 'dismissed' } })}
      tertiaryAction={{
        label: t('modal.approval.deny'),
        onClick: () => approve(approval.kind, approval.id, { deny: { reason: 'denied by user' } }),
      }}
      secondaryAction={{
        label: t('modal.approval.always'),
        onClick: () => approve(approval.kind, approval.id, 'approve_for_session'),
      }}
      primaryAction={{
        label: t('modal.approval.allow'),
        onClick: () => approve(approval.kind, approval.id, 'approve'),
        autoFocus: true,
      }}
    >
      {approval.toolName && (
        <div className={s.codeBlock}>{approval.toolName}</div>
      )}
      {approval.argsSummary && (
        <pre className={s.argsBlock}>{approval.argsSummary}</pre>
      )}
      <p className={s.hint}>
        {t('modal.approval.hint')}
      </p>
    </ModalShell>
  );
}