/**
 * Memory —— 持久记忆管理面板 (B11-01).
 *
 * 通过 `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`
 * 与后端同步。支持 scope (global / project / session) 过滤,
 * 内联编辑 + 删除 + 新增表单。
 */
import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Brain, Plus, Trash2, Save, X } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, EmptyState, Spinner } from '@/features/design-system';
import {
  reflect_add_memory,
  reflect_list_memory,
  reflect_remove_memory,
  type ReflectMemoryEntry,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';
import s from './MemoryView.module.css';

type Scope = 'global' | 'project' | 'session' | 'all';

const SCOPE_LABELS: Record<string, string> = {
  all: 'All',
  global: 'Global',
  project: 'Project',
  session: 'Session',
};

const FILTERS: Scope[] = ['all', 'global', 'project', 'session'];

export function MemoryView() {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);
  const [filter, setFilter] = useState<Scope>('all');
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [newScope, setNewScope] = useState<'global' | 'project' | 'session'>('project');
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');

  const memQ = useQuery({
    queryKey: ['memory'],
    queryFn: () => reflect_list_memory(),
    staleTime: 30_000,
  });

  const entries = memQ.data ?? [];
  const filtered = filter === 'all' ? entries : entries.filter((e) => e.scope === filter);

  // Count per scope
  const counts = (() => {
    const c: Record<string, number> = { all: entries.length };
    for (const e of entries) {
      c[e.scope] = (c[e.scope] ?? 0) + 1;
    }
    return c;
  })();

  const handleRemove = async (entry: ReflectMemoryEntry) => {
    try {
      await reflect_remove_memory(entry.scope, entry.key);
      pushToast({ kind: 'success', message: `Removed ${entry.key}` });
      await qc.invalidateQueries({ queryKey: ['memory'] });
    } catch (e) {
      pushToast({ kind: 'error', message: `Remove failed: ${(e as Error).message}` });
    }
  };

  const handleEditSave = async (entry: ReflectMemoryEntry) => {
    try {
      await reflect_add_memory(entry.scope, entry.key, editValue);
      pushToast({ kind: 'success', message: `Updated ${entry.key}` });
      setEditingKey(null);
      await qc.invalidateQueries({ queryKey: ['memory'] });
    } catch (e) {
      pushToast({ kind: 'error', message: `Update failed: ${(e as Error).message}` });
    }
  };

  const handleAddNew = async () => {
    if (!newKey.trim()) {
      pushToast({ kind: 'warn', message: 'Key is required.' });
      return;
    }
    try {
      await reflect_add_memory(newScope, newKey.trim(), newValue.trim());
      pushToast({ kind: 'success', message: `Added ${newKey}` });
      setNewKey('');
      setNewValue('');
      setShowForm(false);
      await qc.invalidateQueries({ queryKey: ['memory'] });
    } catch (e) {
      pushToast({ kind: 'error', message: `Add failed: ${(e as Error).message}` });
    }
  };

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
            data-active={filter === sc || undefined}
            onClick={() => setFilter(sc)}
            data-testid={`memory-filter-${sc}`}
          >
            {SCOPE_LABELS[sc]} <span className={s.count}>{counts[sc] ?? 0}</span>
          </button>
        ))}
        <button
          type="button"
          className={s.addBtn}
          onClick={() => setShowForm((v) => !v)}
          data-testid="memory-add-btn"
        >
          {showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
          {showForm ? 'Cancel' : 'Add'}
        </button>
      </div>

      {/* Add form */}
      {showForm && (
        <Card level="outlined" padding="sm" className={s.addForm}>
          <div className={s.formRow}>
            <select
              value={newScope}
              onChange={(e) => setNewScope(e.target.value as 'global' | 'project' | 'session')}
              className={s.formSelect}
              data-testid="memory-new-scope"
            >
              <option value="global">Global</option>
              <option value="project">Project</option>
              <option value="session">Session</option>
            </select>
            <input
              value={newKey}
              onChange={(e) => setNewKey(e.target.value)}
              className={s.formInput}
              placeholder="Key (e.g. preference)"
              data-testid="memory-new-key"
            />
            <input
              value={newValue}
              onChange={(e) => setNewValue(e.target.value)}
              className={s.formInput}
              placeholder="Value"
              data-testid="memory-new-value"
            />
            <button type="button" className={s.saveBtn} onClick={handleAddNew} data-testid="memory-add-confirm">
              <Icon icon={Save} size={12} /> Save
            </button>
          </div>
        </Card>
      )}

      {/* List */}
      {memQ.isLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : memQ.error ? (
        <Card level="flat" padding="none">
          <EmptyState icon={<Icon icon={Brain} />} title="Failed to load memory" description="Check the agent backend and try again." />
        </Card>
      ) : filtered.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Brain} />}
            title="No memory entries"
            description="Add a key above or let the agent learn your preferences through chat."
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="memory-list">
          {filtered.map((entry) => (
            <MemoryRow
              key={`${entry.scope}:${entry.key}`}
              entry={entry}
              isEditing={editingKey === `${entry.scope}:${entry.key}`}
              editValue={editValue}
              onEdit={(v) => {
                setEditingKey(`${entry.scope}:${entry.key}`);
                setEditValue(v);
              }}
              onSave={() => void handleEditSave(entry)}
              onCancel={() => setEditingKey(null)}
              onRemove={() => void handleRemove(entry)}
            />
          ))}
        </div>
      )}
    </PageShell>
  );
}

function MemoryRow({
  entry,
  isEditing,
  editValue,
  onEdit,
  onSave,
  onCancel,
  onRemove,
}: {
  entry: ReflectMemoryEntry;
  isEditing: boolean;
  editValue: string;
  onEdit: (v: string) => void;
  onSave: () => void;
  onCancel: () => void;
  onRemove: () => void;
}) {
  return (
    <Card level="outlined" padding="sm" className={s.row} data-testid={`memory-entry-${entry.key}`}>
      <div className={s.rowLeft}>
        <Badge variant="neutral">{entry.scope}</Badge>
        <code className={s.key}>{entry.key}</code>
      </div>
      {isEditing ? (
        <div className={s.editRow}>
          <textarea
            value={editValue}
            onChange={(e) => onEdit(e.target.value)}
            className={s.editInput}
            rows={2}
            autoFocus
            data-testid="memory-edit-input"
          />
          <div className={s.editActions}>
            <button type="button" className={s.saveBtnSmall} onClick={onSave} data-testid="memory-edit-save">
              <Icon icon={Save} size={11} /> Save
            </button>
            <button type="button" className={s.cancelBtnSmall} onClick={onCancel}>
              <Icon icon={X} size={11} />
            </button>
          </div>
        </div>
      ) : (
        <div className={s.rowRight}>
          <span className={s.value} title={entry.value}>{entry.value}</span>
          <div className={s.actions}>
            <button type="button" className={s.actionBtn} onClick={() => onEdit(entry.value)} title="Edit">
              <Icon icon={Plus} size={11} />
            </button>
            <button type="button" className={s.actionBtn} onClick={onRemove} title="Delete" style={{ color: '#f87171' }}>
              <Icon icon={Trash2} size={11} />
            </button>
          </div>
        </div>
      )}
    </Card>
  );
}