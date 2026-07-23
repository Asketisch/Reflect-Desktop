/**
 * ApprovalHistory —— 已处理 approval 列表(B7-03)。
 *
 * 取代模态审批流的"history 槽位"——把过去 N 条 approval 决策(time / tool /
 * decision)以时间倒序列在 Inspector / Settings → Permissions。
 */
import { useMemo } from 'react';
import { CheckCircle2, XCircle, ShieldOff, History } from 'lucide-react';
import { Icon } from '@/features/design-system';
import s from './ApprovalHistory.module.css';

export interface ApprovalRecordView {
  id: string;
  toolName: string;
  decision: 'approve' | 'approve_for_session' | 'deny';
  reason?: string;
  decidedAt: number;
}

export interface ApprovalHistoryProps {
  records: ApprovalRecordView[];
  emptyMessage?: string;
  limit?: number;
}

const ICON_FOR: Record<ApprovalRecordView['decision'], typeof CheckCircle2> = {
  approve: CheckCircle2,
  approve_for_session: ShieldOff,
  deny: XCircle,
};

export function ApprovalHistory({ records, emptyMessage = 'No approvals yet.', limit = 50 }: ApprovalHistoryProps) {
  const sorted = useMemo(
    () => [...records].sort((a, b) => b.decidedAt - a.decidedAt).slice(0, limit),
    [records, limit],
  );

  if (sorted.length === 0) {
    return (
      <div className={s.empty} data-testid="approval-history-empty">
        <Icon icon={History} size={14} /> {emptyMessage}
      </div>
    );
  }

  return (
    <ul className={s.list} data-testid="approval-history">
      {sorted.map((r) => {
        const Icon2 = ICON_FOR[r.decision];
        return (
          <li
            key={r.id}
            className={s.row}
            data-decision={r.decision}
            data-testid={`approval-record-${r.id}`}
          >
            <Icon icon={Icon2} size={13} className={s[`accent_${r.decision}`]} />
            <code className={s.tool}>{r.toolName}</code>
            <span className={s.decision}>{r.decision}</span>
            {r.reason && <span className={s.reason}>— {r.reason}</span>}
            <time className={s.time}>{new Date(r.decidedAt).toLocaleTimeString()}</time>
          </li>
        );
      })}
    </ul>
  );
}
