/**
 * Plan —— Plan mode viewer + approval workflow（CSS Modules 版）。
 *
 * Plan 审批三选一(对齐 `reflect_protocol::PlanApprovalChoice`):
 *   - Auto Mode(auto_mode):切到 AcceptEdits,自动批准编辑/写入类。
 *   - Manual Approve(manual_approve):切到 Prompt,逐工具审批(旧行为)。
 *   - Revise(revise):留在 plan 模式,用户输入反馈继续 plan。
 */
import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ClipboardList, Check, Pencil, Zap } from 'lucide-react';
import { reflect_plan_approval } from '@/utils/commands';
import type { PlanApprovalChoice } from '@/utils/types';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Button, Icon, EmptyState } from '@/features/design-system';
import s from './PlanView.module.css';

interface PlanEvent {
  id: string;
  type: 'plan_request' | 'plan_ready' | 'plan_approved' | 'plan_rejected';
  task: string;
  steps: string[];
  status: 'pending' | 'approved' | 'rejected';
}

const STUB_PLANS: PlanEvent[] = [
  {
    id: 'plan-1',
    type: 'plan_ready',
    task: 'Refactor authentication module',
    steps: ['Analyze current auth flow', 'Update JWT middleware', 'Add refresh token support', 'Write tests'],
    status: 'pending',
  },
];

export function PlanView() {
  const [plans, setPlans] = useState<PlanEvent[]>(STUB_PLANS);
  const qc = useQueryClient();

  // 统一的 plan 审批 mutation:接收 PlanApprovalChoice 三选一。
  const planApprovalMutation = useMutation({
    mutationFn: async ({ id, choice }: { id: string; choice: PlanApprovalChoice }) =>
      reflect_plan_approval(id, choice),
    onSuccess: (_, { id, choice }) => {
      // auto_mode / manual_approve 视为「通过」(切到对应 mode);revise 视为「打回修改」。
      const status: PlanEvent['status'] = choice === 'revise' ? 'rejected' : 'approved';
      setPlans((prev) => prev.map((p) => (p.id === id ? { ...p, status } : p)));
      qc.invalidateQueries({ queryKey: ['plan'] });
    },
  });

  return (
    <PageShell
      icon={ClipboardList}
      title="Plan Mode"
      subtitle={
        <>
          Review and approve plans submitted by the agent. <Badge variant="warning">sample data</Badge>
        </>
      }
      width="md"
    >
      {plans.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={ClipboardList} />}
            title="No plans pending"
            description="Use /plan in chat to create one."
          />
        </Card>
      ) : (
        <div className={s.list}>
          {plans.map((plan) => (
            <Card key={plan.id} level="outlined" padding="lg" className={s.planCard} data-status={plan.status}>
              <div className={s.header}>
                <div className={s.titleRow}>
                  <PlanStatusBadge status={plan.status} />
                  <span className={s.task}>{plan.task}</span>
                </div>
              </div>
              <ol className={s.steps}>
                {plan.steps.map((step, i) => (
                  <li key={i} className={s.step}>
                    <span className={s.stepNum}>{i + 1}</span>
                    <span className={s.stepText}>{step}</span>
                  </li>
                ))}
              </ol>
              {plan.status === 'pending' && (
                <div className={s.actions}>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => planApprovalMutation.mutate({ id: plan.id, choice: 'auto_mode' })}
                    loading={planApprovalMutation.isPending}
                    leftIcon={<Icon icon={Zap} size={14} />}
                  >
                    Auto Mode
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => planApprovalMutation.mutate({ id: plan.id, choice: 'manual_approve' })}
                    loading={planApprovalMutation.isPending}
                    leftIcon={<Icon icon={Check} size={14} />}
                  >
                    Manual Approve
                  </Button>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() => planApprovalMutation.mutate({ id: plan.id, choice: 'revise' })}
                    loading={planApprovalMutation.isPending}
                    leftIcon={<Icon icon={Pencil} size={14} />}
                  >
                    Revise
                  </Button>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}
    </PageShell>
  );
}

function PlanStatusBadge({ status }: { status: PlanEvent['status'] }) {
  if (status === 'approved') return <Badge variant="success" solid>approved</Badge>;
  if (status === 'rejected') return <Badge variant="danger" solid>rejected</Badge>;
  return <Badge variant="warning" solid>pending</Badge>;
}
