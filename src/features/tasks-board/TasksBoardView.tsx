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
  Tooltip,
  type SegmentedOption,
} from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
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
  const { t } = useI18n();
  // Custom list id input visibility (lets the user type an arbitrary list id
  // not backed by a team).
  const [showListPicker, setShowListPicker] = useState(false);

  return (
    <PageShell
      icon={FolderKanban}
      title={t('tasks.title')}
      subtitle={t('tasks.subtitle')}
      width="lg"
      actions={
        <>
          <SegmentedControl
            size="sm"
            options={VIEW_OPTIONS}
            value={ctrl.view}
            onChange={ctrl.setView}
          />
          <Tooltip label={t('tasks.switchList')} side="bottom">
            <button
              type="button"
              className={s.listBtn}
              onClick={() => setShowListPicker((v) => !v)}
              data-testid="tasks-list-picker-toggle"
            >
              <Icon icon={FolderKanban} size={12} /> {ctrl.listId}
            </button>
          </Tooltip>
          <button
            type="button"
            className={s.addBtn}
            onClick={ctrl.toggleShowForm}
            data-testid="tasks-add-btn"
          >
            {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
            {ctrl.showForm ? t('tasks.cancel') : t('tasks.add')}
          </button>
        </>
      }
    >
      {/* List picker */}
      {showListPicker && (
        <Card level="flat" padding="sm" className={s.listPicker} data-testid="tasks-list-picker">
          <div className={s.listPickerLabel}>{t('tasks.activeList')}</div>
          <input
            className={s.formInput}
            value={ctrl.listId}
            onChange={(e) => ctrl.setListId(e.target.value)}
            data-testid="tasks-list-id-input"
            placeholder={t('tasks.listPlaceholder')}
          />
          {ctrl.teamNames.length > 0 && (
            <>
              <div className={s.listPickerLabel}>{t('tasks.teams')}</div>
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
            title={t('tasks.failed')}
            description={t('tasks.failedDesc')}
          />
        </Card>
      ) : ctrl.tasks.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={FolderKanban} />}
            title={t('tasks.empty')}
            description={t('tasks.emptyDesc', { listId: ctrl.listId })}
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
