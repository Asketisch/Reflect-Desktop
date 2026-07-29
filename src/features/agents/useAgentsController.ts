/**
 * Agents controller hook (Phase 1 item 3).
 *
 * Owns:
 *   - TanStack `useQuery` for `reflect_list_agent_defs`
 *   - three mutations: save / delete / parse-preview
 *   - editor state (draft def + raw markdown mode toggle)
 *
 * Mirrors the `useMemoryController` / `useTasksBoardController` precedent.
 */
import { useCallback, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_delete_agent_def,
  reflect_list_agent_defs,
  reflect_save_agent_def,
  type ReflectAgentDef,
} from '@/utils/commands';

export type { ReflectAgentDef, ReflectMemoryScope } from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const AGENTS_QUERY_KEY = ['agent-defs'] as const;
const AGENTS_STALE_MS = 30_000;

/** Empty draft for the create form. */
export function emptyAgentDraft(): ReflectAgentDef {
  return {
    name: '',
    description: '',
    spawnable: false,
    readonly: false,
    tools: [],
    disallowed_tools: [],
    model: null,
    max_turns: null,
    max_result_chars: null,
    memory: [],
    mcp_collections: [],
    system_prompt: '',
  };
}

export interface AgentsController {
  // Query
  defs: ReflectAgentDef[];
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Editor state
  draft: ReflectAgentDef | null;
  isEditing: boolean;
  beginCreate: () => void;
  beginEdit: (def: ReflectAgentDef) => void;
  cancelEdit: () => void;
  patchDraft: (patch: Partial<ReflectAgentDef>) => void;

  // Mutations
  save: () => Promise<void>;
  remove: (def: ReflectAgentDef) => Promise<void>;

  isMutating: boolean;
}

export function useAgentsController(): AgentsController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);

  const [draft, setDraft] = useState<ReflectAgentDef | null>(null);

  const defsQ = useQuery({
    queryKey: [...AGENTS_QUERY_KEY],
    queryFn: () => reflect_list_agent_defs(),
    staleTime: AGENTS_STALE_MS,
  });

  const invalidate = useCallback(
    () => qc.invalidateQueries({ queryKey: [...AGENTS_QUERY_KEY] }),
    [qc],
  );

  const saveMut = useMutation({
    mutationFn: (def: ReflectAgentDef) => reflect_save_agent_def(def),
    onSuccess: () => void invalidate(),
  });

  const deleteMut = useMutation({
    mutationFn: ({ name }: { name: string }) => reflect_delete_agent_def(name),
    onSuccess: () => void invalidate(),
  });

  const beginCreate = useCallback(() => setDraft(emptyAgentDraft()), []);
  const beginEdit = useCallback((def: ReflectAgentDef) => setDraft({ ...def }), []);
  const cancelEdit = useCallback(() => setDraft(null), []);

  const patchDraft = useCallback((patch: Partial<ReflectAgentDef>) => {
    setDraft((prev) => (prev ? { ...prev, ...patch } : prev));
  }, []);

  const save = useCallback(async () => {
    if (!draft) return;
    const name = draft.name.trim();
    const description = draft.description.trim();
    if (!name) {
      pushToast({ kind: 'warn', message: 'Name is required.' });
      return;
    }
    if (!description) {
      pushToast({ kind: 'warn', message: 'Description is required.' });
      return;
    }
    try {
      await saveMut.mutateAsync({ ...draft, name, description });
      pushToast({ kind: 'success', message: `Saved agent “${name}”` });
      setDraft(null);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Save failed: ${(e as Error).message}`,
      });
    }
  }, [draft, pushToast, saveMut]);

  const remove = useCallback(
    async (def: ReflectAgentDef) => {
      try {
        const ok = await deleteMut.mutateAsync({ name: def.name });
        pushToast({
          kind: ok ? 'success' : 'warn',
          message: ok ? `Deleted ${def.name}` : `${def.name} not found`,
        });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Delete failed: ${(e as Error).message}`,
        });
      }
    },
    [deleteMut, pushToast],
  );

  const defs = useMemo(() => defsQ.data ?? [], [defsQ.data]);

  return {
    defs,
    loading: defsQ.isLoading,
    error: defsQ.error,
    refetch: defsQ.refetch,

    draft,
    isEditing: draft !== null,
    beginCreate,
    beginEdit,
    cancelEdit,
    patchDraft,

    save,
    remove,

    isMutating: saveMut.isPending || deleteMut.isPending,
  };
}

/** Comma-separated helper for the tools/disallowed_tools list editors. */
export function parseCsv(input: string): string[] {
  return input
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

export function joinCsv(list: string[]): string {
  return list.join(', ');
}
