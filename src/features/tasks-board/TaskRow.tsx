/**
 * TaskRow —— 列表视图中的单任务行。
 *
 * 展示 id、主题、状态徽标、认领人和按状态区分的操作
 * （claim / advance / delete）。纯展示组件；所有行为通过 props 传入。
 */
import { CheckCircle, Play, Trash2, Hand } from 'lucide-react';
import type { ReflectTask, ReflectTaskStatus } from '@/utils/commands';
import { Badge, Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { STATUS_LABELS } from './useTasksBoardController';
import s from './TasksBoardView.module.css';

const STATUS_VARIANT: Record<ReflectTaskStatus, 'neutral' | 'accent' | 'success' | 'danger'> = {
  pending: 'neutral',
  in_progress: 'accent',
  completed: 'success',
  deleted: 'danger',
};

export interface TaskRowProps {
  task: ReflectTask;
  onClaim: (task: ReflectTask) => void;
  onAdvance: (task: ReflectTask, status: ReflectTaskStatus) => void;
  onRemove: (task: ReflectTask) => void;
}

export function TaskRow({ task, onClaim, onAdvance, onRemove }: TaskRowProps) {
  const { t } = useI18n();
  return (
    <div className={s.row} data-testid={`task-row-${task.id}`}>
      <div className={s.rowMain}>
        <span className={s.rowId}>#{task.id}</span>
        <span className={s.rowSubject}>{task.subject}</span>
        {task.claimed_by && (
          <span className={s.rowClaimer} title={t('tasks.claimed_by')}>
            <Icon icon={Hand} size={11} /> {task.claimed_by}
          </span>
        )}
      </div>
      <div className={s.rowSide}>
        <Badge variant={STATUS_VARIANT[task.status]}>{STATUS_LABELS[task.status]}</Badge>
        <div className={s.rowActions}>
          {task.status === 'pending' && (
            <>
              <button
                type="button"
                className={s.actionBtn}
                onClick={() => onClaim(task)}
                title={t('tasks.claim')}
                data-testid={`task-claim-${task.id}`}
              >
                <Icon icon={Hand} size={12} /> {t('tasks.claim')}
              </button>
              <button
                type="button"
                className={s.actionBtn}
                onClick={() => onAdvance(task, 'in_progress')}
                title={t('tasks.start')}
              >
                <Icon icon={Play} size={12} /> {t('tasks.start')}
              </button>
            </>
          )}
          {task.status === 'in_progress' && (
            <button
              type="button"
              className={s.actionBtn}
              onClick={() => onAdvance(task, 'completed')}
              title={t('tasks.complete')}
              data-testid={`task-complete-${task.id}`}
            >
              <Icon icon={CheckCircle} size={12} /> {t('tasks.complete')}
            </button>
          )}
          {task.status !== 'deleted' && (
            <button
              type="button"
              className={`${s.actionBtn} ${s.actionBtnDanger}`}
              onClick={() => onRemove(task)}
              title={t('tasks.delete')}
              data-testid={`task-delete-${task.id}`}
            >
              <Icon icon={Trash2} size={12} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
