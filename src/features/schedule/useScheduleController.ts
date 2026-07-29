/**
 * Schedule controller hook (Phase 1 item 2).
 *
 * Owns:
 *   - TanStack `useQuery` for `reflect_list_schedules` + `reflect_get_schedule_status`
 *   - three mutations: add / update / remove, each with toast + cache invalidation
 *   - create-form UI state (schedule / prompt / name)
 *
 * Mirrors the `useMemoryController` / `useTasksBoardController` precedent.
 */
import { useCallback, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_add_schedule,
  reflect_get_schedule_status,
  reflect_list_schedules,
  reflect_remove_schedule,
  reflect_update_schedule,
  type ReflectCronJob,
  type ReflectScheduleStatus,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const SCHEDULE_QUERY_KEY = ['schedules'] as const;
const SCHEDULE_STALE_MS = 15_000;

export interface ScheduleController {
  // Query
  jobs: ReflectCronJob[];
  status: ReflectScheduleStatus | null;
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Create-form state
  showForm: boolean;
  toggleShowForm: () => void;
  newSchedule: string;
  setNewSchedule: (v: string) => void;
  newPrompt: string;
  setNewPrompt: (v: string) => void;
  newName: string;
  setNewName: (v: string) => void;

  // Mutations
  create: () => Promise<void>;
  toggle: (job: ReflectCronJob) => Promise<void>;
  remove: (job: ReflectCronJob) => Promise<void>;

  isMutating: boolean;
}

export function useScheduleController(): ScheduleController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);

  const [showForm, setShowForm] = useState(false);
  const [newSchedule, setNewSchedule] = useState('0 * * * *');
  const [newPrompt, setNewPrompt] = useState('');
  const [newName, setNewName] = useState('');

  const jobsQ = useQuery({
    queryKey: [...SCHEDULE_QUERY_KEY],
    queryFn: () => reflect_list_schedules(),
    staleTime: SCHEDULE_STALE_MS,
  });

  const statusQ = useQuery({
    queryKey: [...SCHEDULE_QUERY_KEY, 'status'],
    queryFn: () => reflect_get_schedule_status(),
    staleTime: SCHEDULE_STALE_MS,
  });

  const invalidate = useCallback(
    () => qc.invalidateQueries({ queryKey: [...SCHEDULE_QUERY_KEY] }),
    [qc],
  );

  const addMut = useMutation({
    mutationFn: (args: { schedule: string; prompt: string; name?: string }) =>
      reflect_add_schedule({
        schedule: args.schedule,
        prompt: args.prompt,
        name: args.name || null,
      }),
    onSuccess: () => void invalidate(),
  });

  const updateMut = useMutation({
    mutationFn: (args: { id: string; enabled?: boolean }) =>
      reflect_update_schedule({ id: args.id, enabled: args.enabled }),
    onSuccess: () => void invalidate(),
  });

  const removeMut = useMutation({
    mutationFn: ({ id }: { id: string }) => reflect_remove_schedule(id),
    onSuccess: () => void invalidate(),
  });

  const toggleShowForm = useCallback(() => setShowForm((v) => !v), []);

  const create = useCallback(async () => {
    const schedule = newSchedule.trim();
    const prompt = newPrompt.trim();
    if (!schedule) {
      pushToast({ kind: 'warn', message: 'Schedule (cron expr) is required.' });
      return;
    }
    if (!prompt) {
      pushToast({ kind: 'warn', message: 'Prompt is required.' });
      return;
    }
    try {
      await addMut.mutateAsync({
        schedule,
        prompt,
        name: newName.trim() || undefined,
      });
      pushToast({ kind: 'success', message: `Scheduled “${schedule}”` });
      setNewPrompt('');
      setNewName('');
      setShowForm(false);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Schedule failed: ${(e as Error).message}`,
      });
    }
  }, [addMut, newName, newPrompt, newSchedule, pushToast]);

  const toggle = useCallback(
    async (job: ReflectCronJob) => {
      try {
        await updateMut.mutateAsync({ id: job.id, enabled: !job.enabled });
        pushToast({
          kind: 'success',
          message: `${job.enabled ? 'Disabled' : 'Enabled'} ${job.name ?? job.id}`,
        });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Toggle failed: ${(e as Error).message}`,
        });
      }
    },
    [updateMut, pushToast],
  );

  const remove = useCallback(
    async (job: ReflectCronJob) => {
      try {
        await removeMut.mutateAsync({ id: job.id });
        pushToast({ kind: 'success', message: `Removed ${job.name ?? job.id}` });
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Remove failed: ${(e as Error).message}`,
        });
      }
    },
    [removeMut, pushToast],
  );

  return {
    jobs: jobsQ.data ?? [],
    status: statusQ.data ?? null,
    loading: jobsQ.isLoading,
    error: jobsQ.error,
    refetch: jobsQ.refetch,

    showForm,
    toggleShowForm,
    newSchedule,
    setNewSchedule,
    newPrompt,
    setNewPrompt,
    newName,
    setNewName,

    create,
    toggle,
    remove,

    isMutating: addMut.isPending || updateMut.isPending || removeMut.isPending,
  };
}
