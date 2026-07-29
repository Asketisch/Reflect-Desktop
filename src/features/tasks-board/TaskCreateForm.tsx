/**
 * TaskCreateForm —— inline form for creating a task in the active list.
 *
 * Presentational; state lives in `useTasksBoardController`.
 */
import { Plus } from 'lucide-react';
import { Icon } from '@/features/design-system';
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
        placeholder="Subject (required)"
        value={newSubject}
        onChange={(e) => setNewSubject(e.target.value)}
        data-testid="task-create-subject"
        autoFocus
      />
      <input
        className={s.formInput}
        placeholder="Description (optional)"
        value={newDescription}
        onChange={(e) => setNewDescription(e.target.value)}
        data-testid="task-create-description"
      />
      <input
        className={s.formInput}
        placeholder="Owner / claimer (optional)"
        value={newOwner}
        onChange={(e) => setNewOwner(e.target.value)}
        data-testid="task-create-owner"
      />
      <button type="submit" className={s.saveBtn} data-testid="task-create-submit">
        <Icon icon={Plus} size={12} /> Create
      </button>
    </form>
  );
}
