/**
 * Autopilot — automatic task scheduling view.
 *
 * Phase 3 item 10: cron-based automatic task creation and execution.
 */
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Zap, Clock, History } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Button, EmptyState, Spinner, Input } from '@/features/design-system';
import {
  reflect_get_autopilot_config,
  reflect_update_autopilot_config,
  reflect_autopilot_history,
  type ReflectAutopilotConfig,
} from '@/utils/commands';
import { useState } from 'react';
import s from './AutopilotView.module.css';

const DEFAULT_CONFIG: ReflectAutopilotConfig = {
  enabled: false,
  schedule: '',
  taskTemplate: '',
  agent: null,
  maxConcurrent: 1,
};

export function AutopilotView() {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<ReflectAutopilotConfig | null>(null);

  const configQuery = useQuery({
    queryKey: ['autopilot', 'config'],
    queryFn: reflect_get_autopilot_config,
  });

  const historyQuery = useQuery({
    queryKey: ['autopilot', 'history'],
    queryFn: reflect_autopilot_history,
  });

  const updateMutation = useMutation({
    mutationFn: (cfg: ReflectAutopilotConfig) => reflect_update_autopilot_config(cfg),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autopilot', 'config'] });
      setDraft(null);
    },
  });

  const config = draft ?? configQuery.data ?? DEFAULT_CONFIG;
  const isEditing = draft !== null;

  function update<K extends keyof ReflectAutopilotConfig>(key: K, value: ReflectAutopilotConfig[K]) {
    setDraft({ ...config, ...draft, [key]: value });
  }

  return (
    <PageShell
      icon={Zap}
      title="Autopilot"
      subtitle="Automatic task scheduling. Create and execute tasks on a cron schedule."
      width="md"
    >
      {configQuery.isLoading ? (
        <Spinner size={20} />
      ) : (
        <>
          {/* Config card */}
          <Card level="flat" padding="lg" className={s.configCard}>
            <div className={s.configHeader}>
              <div className={s.configTitle}>
                <Icon icon={Zap} size={18} />
                <span>Configuration</span>
              </div>
              <Badge variant={config.enabled ? 'success' : 'neutral'}>
                {config.enabled ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>

            <div className={s.formRow}>
              <label className={s.label}>
                <input
                  type="checkbox"
                  checked={config.enabled}
                  onChange={(e) => update('enabled', e.target.checked)}
                  disabled={!isEditing}
                />
                <span>Enable autopilot</span>
              </label>
            </div>

            <div className={s.formRow}>
              <label className={s.fieldLabel}>Cron schedule</label>
              <Input
                placeholder="0 9 * * 1-5 (weekdays at 9am)"
                value={config.schedule}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => update('schedule', e.target.value)}
                disabled={!isEditing}
              />
            </div>

            <div className={s.formRow}>
              <label className={s.fieldLabel}>Task template</label>
              <textarea
                className={s.textarea}
                placeholder="Prompt template for automatically created tasks..."
                value={config.taskTemplate}
                onChange={(e) => update('taskTemplate', e.target.value)}
                disabled={!isEditing}
                rows={4}
              />
            </div>

            <div className={s.formRow}>
              <label className={s.fieldLabel}>Agent</label>
              <Input
                placeholder="default (optional)"
                value={config.agent ?? ''}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => update('agent', e.target.value || null)}
                disabled={!isEditing}
              />
            </div>

            <div className={s.formRow}>
              <label className={s.fieldLabel}>Max concurrent</label>
              <Input
                type="number"
                value={config.maxConcurrent}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => update('maxConcurrent', parseInt(e.target.value) || 1)}
                disabled={!isEditing}
              />
            </div>

            <div className={s.actions}>
              {isEditing ? (
                <>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => updateMutation.mutate(draft!)}
                    disabled={updateMutation.isPending}
                  >
                    Save
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => setDraft(null)}>
                    Cancel
                  </Button>
                </>
              ) : (
                <Button variant="ghost" size="sm" onClick={() => setDraft(config)}>
                  Edit
                </Button>
              )}
            </div>
          </Card>

          {/* History card */}
          <Card level="flat" padding="lg" className={s.historyCard}>
            <div className={s.configHeader}>
              <div className={s.configTitle}>
                <Icon icon={History} size={18} />
                <span>Run History</span>
              </div>
              {historyQuery.data && historyQuery.data.length > 0 && (
                <Badge variant="neutral">{historyQuery.data.length}</Badge>
              )}
            </div>

            {historyQuery.isLoading ? (
              <Spinner size={16} />
            ) : historyQuery.data?.length === 0 ? (
              <EmptyState
                icon={<Icon icon={Clock} />}
                title="No runs yet"
                description="Autopilot runs will appear here once enabled."
              />
            ) : (
              <div className={s.historyList}>
                {historyQuery.data?.map((run) => (
                  <div key={run.id} className={s.historyItem}>
                    <div className={s.historyMeta}>
                      <Badge
                        variant={
                          run.status === 'success' ? 'success'
                          : run.status === 'failed' ? 'danger'
                          : run.status === 'inProgress' ? 'info'
                          : 'neutral'
                        }
                      >
                        {run.status}
                      </Badge>
                      <span className={s.historyTime}>
                        {new Date(run.executedAtMs).toLocaleString()}
                      </span>
                    </div>
                    {run.error && <span className={s.historyError}>{run.error}</span>}
                  </div>
                ))}
              </div>
            )}
          </Card>
        </>
      )}
    </PageShell>
  );
}