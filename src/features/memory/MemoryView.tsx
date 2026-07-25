/**
 * Memory —— 持久记忆管理面板 (B11-01).
 *
 * 通过 `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`
 * 与后端同步。支持 scope (global / project / session) 过滤,
 * 内联编辑 + 删除 + 新增表单。
 *
 * Refactored 2026-07-25: query/mutation orchestration lives in
 * `useMemoryController`; row and form rendering live in `MemoryRow` /
 * `MemoryAddForm`. This component composes them with the filter bar and
 * empty/error/loading states.
 */
import { Brain, Plus, X } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Icon, EmptyState, Spinner } from '@/features/design-system';
import {
  FILTERS,
  SCOPE_LABELS,
  entryKey,
  useMemoryController,
} from './useMemoryController';
import { MemoryRow } from './MemoryRow';
import { MemoryAddForm } from './MemoryAddForm';
import s from './MemoryView.module.css';

export function MemoryView() {
  const ctrl = useMemoryController();

  return (
    <PageShell
      icon={Brain}
      title="Memory"
      subtitle="Persistent key-value memory across sessions, scoped by context."
      width="lg"
    >
      {/* Filter bar */}
      <div className={s.filterBar}>
        {FILTERS.map((sc) => (
          <button
            key={sc}
            type="button"
            className={s.filterBtn}
            data-active={ctrl.filter === sc || undefined}
            onClick={() => ctrl.setFilter(sc)}
            data-testid={`memory-filter-${sc}`}
          >
            {SCOPE_LABELS[sc]} <span className={s.count}>{ctrl.counts[sc] ?? 0}</span>
          </button>
        ))}
        <button
          type="button"
          className={s.addBtn}
          onClick={ctrl.toggleShowForm}
          data-testid="memory-add-btn"
        >
          {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
          {ctrl.showForm ? 'Cancel' : 'Add'}
        </button>
      </div>

      {/* Add form */}
      {ctrl.showForm && (
        <MemoryAddForm
          newScope={ctrl.newScope}
          newKey={ctrl.newKey}
          newValue={ctrl.newValue}
          setNewScope={ctrl.setNewScope}
          setNewKey={ctrl.setNewKey}
          setNewValue={ctrl.setNewValue}
          onSubmit={() => void ctrl.addNew()}
        />
      )}

      {/* List */}
      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Brain} />}
            title="Failed to load memory"
            description="Check the agent backend and try again."
          />
        </Card>
      ) : ctrl.filtered.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Brain} />}
            title="No memory entries"
            description="Add a key above or let the agent learn your preferences through chat."
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="memory-list">
          {ctrl.filtered.map((entry) => {
            const key = entryKey(entry);
            return (
              <MemoryRow
                key={key}
                entry={entry}
                isEditing={ctrl.editingKey === key}
                editValue={ctrl.editValue}
                onEdit={() => ctrl.beginEdit(entry)}
                onChangeEdit={ctrl.setEditValue}
                onSave={() => void ctrl.saveEdit(entry)}
                onCancel={ctrl.cancelEdit}
                onRemove={() => void ctrl.remove(entry)}
              />
            );
          })}
        </div>
      )}
    </PageShell>
  );
}
