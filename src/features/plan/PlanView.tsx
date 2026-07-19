/**
 * M3.x Plan —— Plan mode viewer + approval workflow。
 *
 * - 显示 plan request / plan ready 事件
 * - Approve / Reject 按钮
 * - M3.x 扩展:plan diff viewer + edit plan + re-run
 */

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reflect_plan_approval } from '@/utils/tauri';

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
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Plan Mode</h1>

      {plans.length === 0 && (
        <p style={{ color: '#888' }}>No plans pending. Use `/plan` in chat to create one.</p>
      )}

      {plans.map((plan) => (
        <div
          key={plan.id}
          style={{
            border: '1px solid #e2e8f0',
            borderRadius: 8,
            padding: 16,
            marginBottom: 16,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <span
              style={{
                padding: '2px 8px',
                borderRadius: 4,
                fontSize: 11,
                background: statusBg(plan.status),
                color: statusFg(plan.status),
              }}
            >
              {plan.status}
            </span>
            <strong style={{ fontSize: 14 }}>{plan.task}</strong>
          </div>

          <ol style={{ paddingLeft: 20, margin: '0 0 16px', fontSize: 13 }}>
            {plan.steps.map((step, i) => (
              <li key={i} style={{ marginBottom: 4 }}>{step}</li>
            ))}
          </ol>

          {plan.status === 'pending' && (
            <div style={{ display: 'flex', gap: 8 }}>
              <button
                onClick={() => approveMutation.mutate(plan.id)}
                disabled={approveMutation.isPending}
                style={{
                  padding: '6px 16px',
                  background: '#22c55e',
                  color: 'white',
                  border: 'none',
                  borderRadius: 6,
                  cursor: 'pointer',
                  fontSize: 13,
                }}
              >
                Approve
              </button>
              <button
                onClick={() => rejectMutation.mutate(plan.id)}
                disabled={rejectMutation.isPending}
                style={{
                  padding: '6px 16px',
                  background: '#ef4444',
                  color: 'white',
                  border: 'none',
                  borderRadius: 6,
                  cursor: 'pointer',
                  fontSize: 13,
                }}
              >
                Reject
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function statusBg(s: string): string {
  switch (s) {
    case 'pending': return '#fef3c7';
    case 'approved': return '#dcfce7';
    case 'rejected': return '#fee2e2';
    default: return '#f1f5f9';
  }
}
function statusFg(s: string): string {
  switch (s) {
    case 'pending': return '#92400e';
    case 'approved': return '#166534';
    case 'rejected': return '#991b1b';
    default: return '#666';
  }
}
