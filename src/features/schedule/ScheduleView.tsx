/**
 * ScheduleView —— Phase 1 item 2 cron schedule UI.
 *
 * Renders cron jobs from `reflect_list_schedules`, supports add / toggle /
 * remove, and shows the scheduler status badge. Orchestration lives in
 * `useScheduleController`.
 *
 * Backend contract: `src/utils/commands/schedule.ts` ↔
 * `src-tauri/src/commands/schedule.rs` ↔ `vendor/reflect-stream::cron`.
 * Driver (30s tick) fires due jobs by injecting their prompt as a
 * `Submission::user_input` into the agent loop.
 */
import { Plus, X, Clock, Power, Trash2 } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Card, EmptyState, Icon, Spinner } from '@/features/design-system';
import { useScheduleController } from './useScheduleController';
import s from './ScheduleView.module.css';

export function ScheduleView() {
  const ctrl = useScheduleController();

  return (
    <PageShell
      icon={Clock}
      title="Schedule"
      subtitle="Cron-driven autonomous triggers. Due jobs fire their prompt into the agent loop."
      width="lg"
      actions={
        <>
          {ctrl.status && (
            <Badge
              variant={ctrl.status.status === 'has_jobs' ? 'success' : 'neutral'}
            >
              {ctrl.status.enabled}/{ctrl.status.total} active
            </Badge>
          )}
          <button
            type="button"
            className={s.addBtn}
            onClick={ctrl.toggleShowForm}
            data-testid="schedule-add-btn"
          >
            {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
            {ctrl.showForm ? 'Cancel' : 'Add'}
          </button>
        </>
      }
    >
      {/* Create form */}
      {ctrl.showForm && (
        <form
          className={s.addForm}
          data-testid="schedule-create-form"
          onSubmit={(e) => {
            e.preventDefault();
            void ctrl.create();
          }}
        >
          <input
            className={s.formInput}
            placeholder="Schedule (cron: min hour dom month dow, e.g. 0 9 * * 1-5)"
            value={ctrl.newSchedule}
            onChange={(e) => ctrl.setNewSchedule(e.target.value)}
            data-testid="schedule-create-schedule"
          />
          <input
            className={s.formInput}
            placeholder="Prompt (required)"
            value={ctrl.newPrompt}
            onChange={(e) => ctrl.setNewPrompt(e.target.value)}
            data-testid="schedule-create-prompt"
          />
          <input
            className={s.formInput}
            placeholder="Name (optional)"
            value={ctrl.newName}
            onChange={(e) => ctrl.setNewName(e.target.value)}
            data-testid="schedule-create-name"
          />
          <button type="submit" className={s.saveBtn} data-testid="schedule-create-submit">
            <Icon icon={Plus} size={12} /> Schedule
          </button>
        </form>
      )}

      {/* Body */}
      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Clock} />}
            title="Failed to load schedules"
            description="Check the agent backend and try again."
          />
        </Card>
      ) : ctrl.jobs.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Clock} />}
            title="No scheduled jobs"
            description="Add a cron job above to trigger the agent on a schedule."
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="schedule-list">
          {ctrl.jobs.map((job) => (
            <div key={job.id} className={s.row} data-testid={`schedule-row-${job.id}`}>
              <div className={s.rowMain}>
                <code className={s.rowSchedule}>{job.schedule}</code>
                <span className={s.rowPrompt}>{job.prompt}</span>
                {job.next_fire && (
                  <span className={s.rowNext} title="next fire (UTC)">
                    next: {new Date(job.next_fire).toLocaleString()}
                  </span>
                )}
              </div>
              <div className={s.rowSide}>
                <Badge variant={job.enabled ? 'success' : 'neutral'}>
                  {job.enabled ? 'enabled' : 'disabled'}
                </Badge>
                <div className={s.rowActions}>
                  <button
                    type="button"
                    className={s.actionBtn}
                    onClick={() => void ctrl.toggle(job)}
                    title={job.enabled ? 'Disable' : 'Enable'}
                    data-testid={`schedule-toggle-${job.id}`}
                  >
                    <Icon icon={Power} size={12} />
                  </button>
                  <button
                    type="button"
                    className={`${s.actionBtn} ${s.actionBtnDanger}`}
                    onClick={() => void ctrl.remove(job)}
                    title="Remove"
                    data-testid={`schedule-remove-${job.id}`}
                  >
                    <Icon icon={Trash2} size={12} />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </PageShell>
  );
}
