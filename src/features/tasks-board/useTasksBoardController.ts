/**
 * Tasks board —— controller hook (Phase 1 multi-agent UI).
 *
 * Encapsulates query / mutation orchestration so the view stays presentational.
 * Owns:
 *   - TanStack `useQuery` for `reflect_list_tasks` (polls the active list)
 *   - `useQuery` for `reflect_list_teams` (drives the list selector: list id =
 *     team name when a team is picked, free text otherwise)
 *   - four mutations: create / claim / update(status) / delete, each with toast
 *     + cache invalidation
 *   - view-mode (list / board) + active list id + create-form UI state.
 *
 * Mirrors the `useMemoryController` shape (2026-07-25 refactor precedent).
 */
import { useCallback, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_claim_task,
  reflect_create_task,
  reflect_delete_task,
  reflect_list_tasks,
  reflect_list_teams,
  reflect_update_task,
  type ReflectTask,
  type ReflectTaskStatus,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const TASKS_QUERY_KEY_BASE = ['tasks'] as const;
const TASKS_STALE_MS = 15_000;
const TEAMS_STALE_MS = 60_000;

/** Default list id when no team / custom list is selected. */
export const DEFAULT_LIST_ID = 'default';

export type BoardView = 'list' | 'board';

export const STATUS_ORDER: ReflectTaskStatus[] = [
  'pending',
  'in_progress',
  'completed',
  'deleted',
];

/** Legacy - use i18n `tasks.*` keys instead. Kept for backwards compat. */
export const STATUS_LABELS: Record<ReflectTaskStatus, string> = {
  pending: 'Pending',
  in_progress: 'In Progress',
  completed: 'Completed',
  deleted: 'Deleted',
};

export interface TasksBoardController {
  // Task query
  tasks: ReflectTask[];
  /** Tasks grouped by status (board view). Excludes soft-deleted. */
  grouped: Record<ReflectTaskStatus, ReflectTask[]>;
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Teams query (drives list selector)
  teamNames: string[];

  // View + list state
  view: BoardView;
  setView: (v: BoardView) => void;
  listId: string;
  setListId: (id: string) => void;

  // Create-form state
  showForm: boolean;
  toggleShowForm: () => void;
  newSubject: string;
  setNewSubject: (v: string) => void;
  newDescription: string;
  setNewDescription: (v: string) => void;
  newOwner: string;
  setNewOwner: (v: string) => void;

  // Mutations
  create: () => Promise<void>;
  claim: (task: ReflectTask) => Promise<void>;
  advance: (task: ReflectTask, status: ReflectTaskStatus) => Promise<void>;
  remove: (task: ReflectTask) => Promise<void>;

  isMutating: boolean;
}

function tasksQueryKey(listId: string): readonly unknown[] {
  return [...TASKS_QUERY_KEY_BASE, listId] as const;
}

export function useTasksBoardController(): TasksBoardController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);

  const [view, setView] = useState<BoardView>('list');
  const [listId, setListId] = useState<string>(DEFAULT_LIST_ID);
  const [showForm, setShowForm] = useState(false);
  const [newSubject, setNewSubject] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newOwner, setNewOwner] = useState('');

  const tasksQ = useQuery({
    queryKey: tasksQueryKey(listId),
    queryFn: () => reflect_list_tasks(listId),
    staleTime: TASKS_STALE_MS,
  });

  const teamsQ = useQuery({
    queryKey: ['teams'],
    queryFn: () => reflect_list_teams(),
    staleTime: TEAMS_STALE_MS,
  });

  const invalidateTasks = useCallback(
    () => qc.invalidateQueries({ queryKey: [...TASKS_QUERY_KEY_BASE] }),
    [qc],
  );

  const createMut = useMutation({
    mutationFn: (args: {
      list: string;
      subject: string;
      description: string;
      owner?: string;
    }) =>
      reflect_create_task({
        list: args.list,
        subject: args.subject,
        description: args.description,
        owner: args.owner ?? null,
      }),
    onSuccess: () => void invalidateTasks(),
  });

  const claimMut = useMutation({
    mutationFn: ({ list, claimer }: { list: string; claimer: string }) =>
      reflect_claim_task(list, claimer),
    onSuccess: () => void invalidateTasks(),
  });

  const updateMut = useMutation({
    mutationFn: ({
      list,
      id,
      patch,
    }: {
      list: string;
      id: number;
      patch: { status: ReflectTaskStatus };
    }) => reflect_update_task(list, id, patch),
    onSuccess: () => void invalidateTasks(),
  });

  const deleteMut = useMutation({
    mutationFn: ({ list, id }: { list: string; id: number }) =>
      reflect_delete_task(list, id),
    onSuccess: () => void invalidateTasks(),
  });

  const tasks = useMemo(() => tasksQ.data ?? [], [tasksQ.data]);

  const grouped = useMemo<Record<ReflectTaskStatus, ReflectTask[]>>(() => {
    const init: Record<ReflectTaskStatus, ReflectTask[]> = {
      pending: [],
      in_progress: [],
      completed: [],
      deleted: [],
    };
    for (const t of tasks) {
      init[t.status].push(t);
    }
    return init;
  }, [tasks]);

  const teamNames = useMemo(
    () => (teamsQ.data ?? []).map((tm) => tm.name).sort(),
    [teamsQ.data],
  );

  const toggleShowForm = useCallback(() => setShowForm((v) => !v), []);

  const create = useCallback(async () => {
    const subject = newSubject.trim();
    if (!subject) {
      pushToast({ kind: 'warn', message: 'Subject is required.' });
      return;
    }
    try {
      await createMut.mutateAsync({
        list: listId,
        subject,
        description: newDescription.trim(),
        owner: newOwner.trim() || undefined,
      });
      pushToast({ kind: 'success', message: `Created “${subject}”` });
      setNewSubject('');
      setNewDescription('');
      setNewOwner('');
      setShowForm(false);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Create failed: ${(e as Error).message}`,
      });
    }
  }, [createMut, listId, newDescription, newOwner, newSubject, pushToast]);

  const claim = useCallback(
    async (task: ReflectTask) => {
      // Claim under the task's own list (resilient if user flipped listId).
      const claimer = newOwner.trim() || 'desktop-user';
      try {
        const claimed = await claimMut.mutateAsync({
          list: task.list_id,
          claimer,
        });
        if (claimed) {
          pushToast({
            kind: 'success',
            message: `Claimed #${claimed.id} “${claimed.subject}”`,
          });
        } else {
          pushToast({
            kind: 'warn',
            message: 'No claimable task (queue empty or all blocked).',
          });
        }
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Claim failed: ${(e as Error).message}`,
        });
      }
    },
    [claimMut, newOwner, pushToast],
  );

  const advance = useCallback(
    async (task: ReflectTask, status: ReflectTaskStatus) => {
      try {
        await updateMut.mutateAsync({
          list: task.list_id,
          id: task.id,
          patch: { status },
        });
        pushToast({
          kind: 'success',
          message: `#${task.id} → ${STATUS_LABELS[status]}`,
        });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Update failed: ${(e as Error).message}`,
        });
      }
    },
    [updateMut, pushToast],
  );

  const remove = useCallback(
    async (task: ReflectTask) => {
      try {
        await deleteMut.mutateAsync({ list: task.list_id, id: task.id });
        pushToast({ kind: 'success', message: `Deleted #${task.id}` });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Delete failed: ${(e as Error).message}`,
        });
      }
    },
    [deleteMut, pushToast],
  );

  return {
    tasks,
    grouped,
    loading: tasksQ.isLoading,
    error: tasksQ.error,
    refetch: tasksQ.refetch,

    teamNames,

    view,
    setView,
    listId,
    setListId,

    showForm,
    toggleShowForm,
    newSubject,
    setNewSubject,
    newDescription,
    setNewDescription,
    newOwner,
    setNewOwner,

    create,
    claim,
    advance,
    remove,

    isMutating:
      createMut.isPending ||
      claimMut.isPending ||
      updateMut.isPending ||
      deleteMut.isPending,
  };
}
