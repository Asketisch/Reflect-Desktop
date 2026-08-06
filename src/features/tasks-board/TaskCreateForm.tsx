/**
 * TaskCreateForm —— inline form for creating a task in the active list.
 *
 * Presentational; state lives in `useTasksBoardController`.
 */
import { Plus } from 'lucide-react';
import { Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import s from './TasksBoardView.module.css';

export interface TaskCreateFormProps {
  newSubject: string;
  newDescription: string;
  newOwner: string;
  setNewSubject: (v: string) => void;
  setNewDescription: (v: string) => void;
  setNewOwner: (v: string) => void;
  onSubmit: () => void;
}

export function TaskCreateForm({
  newSubject,
  newDescription,
  newOwner,
  setNewSubject,
  setNewDescription,
  setNewOwner,
  onSubmit,
}: TaskCreateFormProps) {
  const { t } = useI18n();
  return (
    <form
      className={s.addForm}
      data-testid="task-create-form"
      onSubmit={(e) => {
        e.preventDefault();
        void onSubmit();
      }}
    >
      <input
        className={s.formInput}
        placeholder={t('tasks.subject')}
        value={newSubject}
        onChange={(e) => setNewSubject(e.target.value)}
        data-testid="task-create-subject"
        autoFocus
      />
      <input
        className={s.formInput}
        placeholder={t('tasks.description')}
        value={newDescription}
        onChange={(e) => setNewDescription(e.target.value)}
        data-testid="task-create-description"
      />
      <input
        className={s.formInput}
        placeholder={t('tasks.owner')}
        value={newOwner}
        onChange={(e) => setNewOwner(e.target.value)}
        data-testid="task-create-owner"
      />
      <button type="submit" className={s.saveBtn} data-testid="task-create-submit">
        <Icon icon={Plus} size={12} /> {t('tasks.create')}
      </button>
    </form>
  );
}
