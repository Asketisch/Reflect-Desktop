/**
 * Agents controller hook（阶段 1 任务 3）。
 *
 * 职责：
 *   - TanStack `useQuery` 用于 `reflect_list_agent_defs`
 *   - 三个 mutation：save / delete / parse-preview
 *   - 编辑器状态（草稿 def + 原始 markdown 模式切换）
 *
 * 参考 `useMemoryController` / `useTasksBoardController` 的模式。
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

/** 创建表单的空草稿。 */
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
    memory: [],
    mcp_collections: [],
    system_prompt: '',
  };
}

export interface AgentsController {
  // 查询
  defs: ReflectAgentDef[];
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // 编辑器状态
  draft: ReflectAgentDef | null;
  isEditing: boolean;
  beginCreate: () => void;
  beginEdit: (def: ReflectAgentDef) => void;
  cancelEdit: () => void;
  patchDraft: (patch: Partial<ReflectAgentDef>) => void;

  // 突变操作
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

/** tools / disallowed_tools 列表编辑器的逗号分隔辅助函数。 */
export function parseCsv(input: string): string[] {
  return input
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

export function joinCsv(list: string[]): string {
  return list.join(', ');
}
