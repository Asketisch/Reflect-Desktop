/**
 * Plan —— Plan mode viewer + approval workflow（CSS Modules 版）。
 */
import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ClipboardList, Check, X } from 'lucide-react';
import { reflect_plan_approval } from '@/utils/commands';
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

  const approveMutation = useMutation({
    mutationFn: async (id: string) => reflect_plan_approval(id, 'approve'),
    onSuccess: (_, id) => {
      setPlans((prev) => prev.map((p) => (p.id === id ? { ...p, status: 'approved' as const } : p)));
      qc.invalidateQueries({ queryKey: ['plan'] });
    },
  });

  const rejectMutation = useMutation({
    mutationFn: async (id: string) => reflect_plan_approval(id, { deny: { reason: 'rejected by user' } }),
    onSuccess: (_, id) => {
      setPlans((prev) => prev.map((p) => (p.id === id ? { ...p, status: 'rejected' as const } : p)));
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
                    variant="danger"
                    size="sm"
                    onClick={() => rejectMutation.mutate(plan.id)}
                    loading={rejectMutation.isPending}
                    leftIcon={<Icon icon={X} size={14} />}
                  >
                    Reject
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => approveMutation.mutate(plan.id)}
                    loading={approveMutation.isPending}
                    leftIcon={<Icon icon={Check} size={14} />}
                  >
                    Approve
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
