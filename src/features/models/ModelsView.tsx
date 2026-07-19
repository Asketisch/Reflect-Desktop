/**
 * Models —— 阶段 4:从真实 config + agent status 读数据。
 *
 * - 当前 model 来自 `reflect_agent_status`(后端 resolved_model_spec)。
 * - reasoning effort 经 `reflect_set_effort` 下发(后端期望 low/medium/high 小写)。
 */
import { useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { reflect_agent_status, reflect_set_effort } from '@/utils/tauri';

const REASONING_EFFORTS = ['low', 'medium', 'high'] as const;
type Effort = (typeof REASONING_EFFORTS)[number];

export function ModelsView() {
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
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Models</h1>

      <section
        style={{
          marginBottom: 24,
          padding: 12,
          borderRadius: 6,
          background: hasModel ? '#dcfce7' : '#fef3c7',
          border: `1px solid ${hasModel ? '#22c55e' : '#f59e0b'}`,
        }}
      >
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>Current model</h2>
        {statusQuery.isLoading ? (
          <p style={{ fontSize: 13 }}>Loading…</p>
        ) : (
          <code style={{ fontSize: 14 }}>{status?.model ?? '(unknown)'}</code>
        )}
        {status?.workspace && (
          <p style={{ fontSize: 11, color: '#888', marginTop: 6 }}>
            workspace: {status.workspace}
          </p>
        )}
        {!hasModel && (
          <p style={{ fontSize: 12, color: '#92400e', marginTop: 8 }}>
            ⚠ No provider configured. Go to Settings to set an API key.
          </p>
        )}
      </section>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>Reasoning Effort</h2>
        <p style={{ fontSize: 11, color: '#888', marginTop: 0, marginBottom: 8 }}>
          切换会话内 reasoning 强度(经 Op::SetEffort)。model spec 在 Settings 页编辑。
        </p>
        <div style={{ display: 'flex', gap: 8 }}>
          {REASONING_EFFORTS.map((e) => (
            <button
              key={e}
              onClick={() => setEffort(e)}
              style={{
                padding: '6px 14px',
                border: '1px solid',
                borderColor: effort === e ? '#3b82f6' : '#e2e8f0',
                background: effort === e ? '#dbeafe' : 'white',
                borderRadius: 6,
                cursor: 'pointer',
                fontSize: 13,
              }}
            >
              {e}
            </button>
          ))}
        </div>
        <button
          onClick={() => effortMutation.mutate(effort)}
          disabled={effortMutation.isPending}
          style={{
            marginTop: 12,
            padding: '8px 16px',
            background: effortMutation.isPending ? '#93c5fd' : '#3b82f6',
            color: 'white',
            border: 'none',
            borderRadius: 6,
            cursor: effortMutation.isPending ? 'wait' : 'pointer',
            fontSize: 13,
          }}
        >
          {effortMutation.isPending ? 'Saving…' : saved ? 'Saved ✓' : 'Apply Effort'}
        </button>
      </section>
    </div>
  );
}
