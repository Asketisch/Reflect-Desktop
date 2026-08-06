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
import { useI18n } from '@/utils/i18n';
import { useScheduleController } from './useScheduleController';
import s from './ScheduleView.module.css';

export function ScheduleView() {
  const ctrl = useScheduleController();
  const { t } = useI18n();

  return (
    <PageShell
      icon={Clock}
      title={t('schedule.title')}
      subtitle={t('schedule.subtitle')}
      width="lg"
      actions={
        <>
          {ctrl.status && (
            <Badge
              variant={ctrl.status.status === 'has_jobs' ? 'success' : 'neutral'}
            >
              {ctrl.status.enabled}/{ctrl.status.total} {t('schedule.active')}
            </Badge>
          )}
          <button
            type="button"
            className={s.addBtn}
            onClick={ctrl.toggleShowForm}
            data-testid="schedule-add-btn"
          >
            {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
            {ctrl.showForm ? t('schedule.cancel') : t('schedule.add')}
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
            placeholder={t('schedule.cron')}
            value={ctrl.newSchedule}
            onChange={(e) => ctrl.setNewSchedule(e.target.value)}
            data-testid="schedule-create-schedule"
          />
          <input
            className={s.formInput}
            placeholder={t('schedule.prompt')}
            value={ctrl.newPrompt}
            onChange={(e) => ctrl.setNewPrompt(e.target.value)}
            data-testid="schedule-create-prompt"
          />
          <input
            className={s.formInput}
            placeholder={t('schedule.name')}
            value={ctrl.newName}
            onChange={(e) => ctrl.setNewName(e.target.value)}
            data-testid="schedule-create-name"
          />
          <button type="submit" className={s.saveBtn} data-testid="schedule-create-submit">
            <Icon icon={Plus} size={12} /> {t('schedule.create')}
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
            title={t('schedule.failed')}
            description={t('schedule.failedDesc')}
          />
        </Card>
      ) : ctrl.jobs.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Clock} />}
            title={t('schedule.empty')}
            description={t('schedule.emptyDesc')}
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
                  <span className={s.rowNext} title={t('schedule.nextFire')}>
                    {new Date(job.next_fire).toLocaleString()}
                  </span>
                )}
              </div>
              <div className={s.rowSide}>
                <Badge variant={job.enabled ? 'success' : 'neutral'}>
                  {job.enabled ? t('schedule.enabled') : t('schedule.disabled')}
                </Badge>
                <div className={s.rowActions}>
                  <button
                    type="button"
                    className={s.actionBtn}
                    onClick={() => void ctrl.toggle(job)}
                    title={job.enabled ? t('schedule.disable') : t('schedule.enable')}
                    data-testid={`schedule-toggle-${job.id}`}
                  >
                    <Icon icon={Power} size={12} />
                  </button>
                  <button
                    type="button"
                    className={`${s.actionBtn} ${s.actionBtnDanger}`}
                    onClick={() => void ctrl.remove(job)}
                    title={t('schedule.remove')}
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
