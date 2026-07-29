/**
 * TasksBoard —— Phase 1 multi-agent task board.
 *
 * Renders tasks from `reflect_list_tasks` in two views (List / Board),
 * supports create / claim / advance-status / delete, and lets the user pick
 * the active list (defaults to "default"; switches to a team name when one is
 * selected). All orchestration lives in `useTasksBoardController`.
 *
 * Backend contract: `src/utils/commands/{tasks,teams}.ts` ↔
 * `src-tauri/src/commands/tasks.rs`. Storage: `~/.reflect/tasks/<list>/` +
 * `~/.reflect/teams/`, shared with the TUI/CLI.
 */
import { useState } from 'react';
import { KanbanSquare, List as ListIcon, Plus, X, FolderKanban } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import {
  Card,
  EmptyState,
  Icon,
  SegmentedControl,
  Spinner,
  type SegmentedOption,
} from '@/features/design-system';
import {
  STATUS_LABELS,
  useTasksBoardController,
  type BoardView,
} from './useTasksBoardController';
import { TaskRow } from './TaskRow';
import { TaskCreateForm } from './TaskCreateForm';
import type { ReflectTaskStatus } from '@/utils/commands';
import s from './TasksBoardView.module.css';

const VIEW_OPTIONS: SegmentedOption<BoardView>[] = [
  { value: 'list', label: <Icon icon={ListIcon} size={12} /> },
  { value: 'board', label: <Icon icon={KanbanSquare} size={12} /> },
];

const BOARD_COLUMNS: ReflectTaskStatus[] = ['pending', 'in_progress', 'completed'];

export function TasksBoardView() {
  const ctrl = useTasksBoardController();
  // Custom list id input visibility (lets the user type an arbitrary list id
  // not backed by a team).
  const [showListPicker, setShowListPicker] = useState(false);

  return (
    <PageShell
      icon={FolderKanban}
      title="Tasks"
      subtitle="Multi-agent task board. Create, claim, advance, and delete tasks across lists."
      width="lg"
      actions={
        <>
          <SegmentedControl
            size="sm"
            options={VIEW_OPTIONS}
            value={ctrl.view}
            onChange={ctrl.setView}
          />
          <button
            type="button"
            className={s.listBtn}
            onClick={() => setShowListPicker((v) => !v)}
            title="Switch list / team"
            data-testid="tasks-list-picker-toggle"
          >
            <Icon icon={FolderKanban} size={12} /> {ctrl.listId}
          </button>
          <button
            type="button"
            className={s.addBtn}
            onClick={ctrl.toggleShowForm}
            data-testid="tasks-add-btn"
          >
            {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
            {ctrl.showForm ? 'Cancel' : 'Add'}
          </button>
        </>
      }
    >
      {/* List picker */}
      {showListPicker && (
        <Card level="flat" padding="sm" className={s.listPicker} data-testid="tasks-list-picker">
          <div className={s.listPickerLabel}>Active list</div>
          <input
            className={s.formInput}
            value={ctrl.listId}
            onChange={(e) => ctrl.setListId(e.target.value)}
            data-testid="tasks-list-id-input"
            placeholder="list id (e.g. default, or a team name)"
          />
          {ctrl.teamNames.length > 0 && (
            <>
              <div className={s.listPickerLabel}>Teams</div>
              <div className={s.teamChips}>
                {ctrl.teamNames.map((name) => (
                  <button
                    key={name}
                    type="button"
                    className={s.teamChip}
                    data-active={ctrl.listId === name || undefined}
                    onClick={() => {
                      ctrl.setListId(name);
                      setShowListPicker(false);
                    }}
                  >
                    {name}
                  </button>
                ))}
              </div>
            </>
          )}
        </Card>
      )}

      {/* Create form */}
      {ctrl.showForm && (
        <TaskCreateForm
          newSubject={ctrl.newSubject}
          newDescription={ctrl.newDescription}
          newOwner={ctrl.newOwner}
          setNewSubject={ctrl.setNewSubject}
          setNewDescription={ctrl.setNewDescription}
          setNewOwner={ctrl.setNewOwner}
          onSubmit={() => void ctrl.create()}
        />
      )}

      {/* Body */}
      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={FolderKanban} />}
            title="Failed to load tasks"
            description="Check the agent backend and try again."
          />
        </Card>
      ) : ctrl.tasks.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={FolderKanban} />}
            title="No tasks"
            description={`List “${ctrl.listId}” is empty. Add one above.`}
          />
        </Card>
      ) : ctrl.view === 'list' ? (
        <div className={s.list} data-testid="tasks-list-view">
          {ctrl.tasks.map((task) => (
            <TaskRow
              key={`${task.list_id}:${task.id}`}
              task={task}
              onClaim={(t) => void ctrl.claim(t)}
              onAdvance={(t, st) => void ctrl.advance(t, st)}
              onRemove={(t) => void ctrl.remove(t)}
            />
          ))}
        </div>
      ) : (
        <div className={s.board} data-testid="tasks-board-view">
          {BOARD_COLUMNS.map((status) => (
            <div key={status} className={s.boardColumn} data-status={status}>
              <div className={s.boardColumnHeader}>
                {STATUS_LABELS[status]}{' '}
                <span className={s.boardColumnCount}>
                  {ctrl.grouped[status].length}
                </span>
              </div>
              <div className={s.boardColumnBody}>
                {ctrl.grouped[status].length === 0 ? (
                  <div className={s.boardEmpty}>—</div>
                ) : (
                  ctrl.grouped[status].map((task) => (
                    <TaskRow
                      key={`${task.list_id}:${task.id}`}
                      task={task}
                      onClaim={(t) => void ctrl.claim(t)}
                      onAdvance={(t, st) => void ctrl.advance(t, st)}
                      onRemove={(t) => void ctrl.remove(t)}
                    />
                  ))
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </PageShell>
  );
}
