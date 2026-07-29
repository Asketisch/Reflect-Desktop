/**
 * TaskRow —— single-task row in the List view.
 *
 * Shows id, subject, status badge, claimer, and per-status actions
 * (claim / advance / delete). Presentational only; all behavior comes via props.
 */
import { CheckCircle, Play, Trash2, Hand } from 'lucide-react';
import type { ReflectTask, ReflectTaskStatus } from '@/utils/commands';
import { Badge, Icon } from '@/features/design-system';
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
  return (
    <div className={s.row} data-testid={`task-row-${task.id}`}>
      <div className={s.rowMain}>
        <span className={s.rowId}>#{task.id}</span>
        <span className={s.rowSubject}>{task.subject}</span>
        {task.claimed_by && (
          <span className={s.rowClaimer} title="claimed by">
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
                title="Claim"
                data-testid={`task-claim-${task.id}`}
              >
                <Icon icon={Hand} size={12} /> Claim
              </button>
              <button
                type="button"
                className={s.actionBtn}
                onClick={() => onAdvance(task, 'in_progress')}
                title="Start"
              >
                <Icon icon={Play} size={12} /> Start
              </button>
            </>
          )}
          {task.status === 'in_progress' && (
            <button
              type="button"
              className={s.actionBtn}
              onClick={() => onAdvance(task, 'completed')}
              title="Complete"
              data-testid={`task-complete-${task.id}`}
            >
              <Icon icon={CheckCircle} size={12} /> Complete
            </button>
          )}
          {task.status !== 'deleted' && (
            <button
              type="button"
              className={`${s.actionBtn} ${s.actionBtnDanger}`}
              onClick={() => onRemove(task)}
              title="Delete"
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
