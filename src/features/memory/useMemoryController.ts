/**
 * Memory —— controller hook (B11-01 refactor).
 *
 * Encapsulates query / mutation orchestration so the view component can stay
 * presentational. Owns:
 *   - the TanStack `useQuery` for `reflect_list_memory`
 *   - the three mutations (`handleRemove` / `handleEditSave` / `handleAddNew`)
 *     including toasts and cache invalidation
 *   - filter + edit + new-form UI state.
 *
 * Extracted from MemoryView (2026-07-25); behavior preserved.
 */
import { useCallback, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_add_memory,
  reflect_list_memory,
  reflect_remove_memory,
  type ReflectMemoryEntry,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const MEMORY_QUERY_KEY = ['memory'] as const;
const MEMORY_STALE_MS = 30_000;

export type Scope = 'global' | 'project' | 'session' | 'all';

export const SCOPE_LABELS: Record<string, string> = {
  all: 'All',
  global: 'Global',
  project: 'Project',
  session: 'Session',
};

export const FILTERS: Scope[] = ['all', 'global', 'project', 'session'];

export const NEW_SCOPES: Array<'global' | 'project' | 'session'> = [
  'global',
  'project',
  'session',
];

export interface MemoryCounts {
  all: number;
  [scope: string]: number;
}

export interface MemoryController {
  // Query
  entries: ReflectMemoryEntry[];
  filtered: ReflectMemoryEntry[];
  counts: MemoryCounts;
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Filter state
  filter: Scope;
  setFilter: (s: Scope) => void;

  // Edit state
  editingKey: string | null;
  editValue: string;
  beginEdit: (entry: ReflectMemoryEntry) => void;
  setEditValue: (v: string) => void;
  cancelEdit: () => void;

  // New-entry form state
  showForm: boolean;
  toggleShowForm: () => void;
  newScope: 'global' | 'project' | 'session';
  setNewScope: (s: 'global' | 'project' | 'session') => void;
  newKey: string;
  setNewKey: (v: string) => void;
  newValue: string;
  setNewValue: (v: string) => void;

  // Mutations
  remove: (entry: ReflectMemoryEntry) => Promise<void>;
  saveEdit: (entry: ReflectMemoryEntry) => Promise<void>;
  addNew: () => Promise<void>;

  // In-flight mutation state (used by callers to disable controls).
  isMutating: boolean;
}

export function useMemoryController(): MemoryController {
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
    queryKey: [...MEMORY_QUERY_KEY],
    queryFn: () => reflect_list_memory(),
    staleTime: MEMORY_STALE_MS,
  });

  const invalidate = useCallback(
    () => qc.invalidateQueries({ queryKey: [...MEMORY_QUERY_KEY] }),
    [qc],
  );

  const addMut = useMutation({
    mutationFn: ({ scope, key, value }: { scope: string; key: string; value: string }) =>
      reflect_add_memory(scope, key, value),
    onSuccess: () => {
      void invalidate();
    },
  });

  const removeMut = useMutation({
    mutationFn: ({ scope, key }: { scope: string; key: string }) =>
      reflect_remove_memory(scope, key),
    onSuccess: () => {
      void invalidate();
    },
  });

  const entries = memQ.data ?? [];
  const filtered = useMemo(
    () => (filter === 'all' ? entries : entries.filter((e) => e.scope === filter)),
    [entries, filter],
  );

  const counts = useMemo<MemoryCounts>(() => {
    const c: MemoryCounts = { all: entries.length };
    for (const e of entries) {
      c[e.scope] = (c[e.scope] ?? 0) + 1;
    }
    return c;
  }, [entries]);

  const beginEdit = useCallback((entry: ReflectMemoryEntry) => {
    setEditingKey(`${entry.scope}:${entry.key}`);
    setEditValue(entry.value);
  }, []);

  const cancelEdit = useCallback(() => setEditingKey(null), []);

  const toggleShowForm = useCallback(() => setShowForm((v) => !v), []);

  const remove = useCallback(
    async (entry: ReflectMemoryEntry) => {
      try {
        await removeMut.mutateAsync({ scope: entry.scope, key: entry.key });
        pushToast({ kind: 'success', message: `Removed ${entry.key}` });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Remove failed: ${(e as Error).message}`,
        });
      }
    },
    [pushToast, removeMut],
  );

  const saveEdit = useCallback(
    async (entry: ReflectMemoryEntry) => {
      try {
        await addMut.mutateAsync({ scope: entry.scope, key: entry.key, value: editValue });
        pushToast({ kind: 'success', message: `Updated ${entry.key}` });
        setEditingKey(null);
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Update failed: ${(e as Error).message}`,
        });
      }
    },
    [addMut, editValue, pushToast],
  );

  const addNew = useCallback(async () => {
    const trimmedKey = newKey.trim();
    if (!trimmedKey) {
      pushToast({ kind: 'warn', message: 'Key is required.' });
      return;
    }
    try {
      await addMut.mutateAsync({
        scope: newScope,
        key: trimmedKey,
        value: newValue.trim(),
      });
      pushToast({ kind: 'success', message: `Added ${trimmedKey}` });
      setNewKey('');
      setNewValue('');
      setShowForm(false);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Add failed: ${(e as Error).message}`,
      });
    }
  }, [addMut, newKey, newScope, newValue, pushToast]);

  return {
    entries,
    filtered,
    counts,
    loading: memQ.isLoading,
    error: memQ.error,
    refetch: memQ.refetch,

    filter,
    setFilter,

    editingKey,
    editValue,
    beginEdit,
    setEditValue,
    cancelEdit,

    showForm,
    toggleShowForm,
    newScope,
    setNewScope,
    newKey,
    setNewKey,
    newValue,
    setNewValue,

    remove,
    saveEdit,
    addNew,

    isMutating: addMut.isPending || removeMut.isPending,
  };
}

export function entryKey(entry: ReflectMemoryEntry): string {
  return `${entry.scope}:${entry.key}`;
}
