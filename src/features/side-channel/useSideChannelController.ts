/**
 * Side-channel controller hook (Phase 2 item 1).
 *
 * Encapsulates query / mutation orchestration:
 *   - TanStack `useQuery` for `reflect_list_side_channels` (live updates
 *     rely on the broadcast event stream once the driver is wired; for
 *     now this is an ad-hoc snapshot refreshed on focus + every mutation).
 *   - two mutations: start / cancel, with toast + invalidation.
 *
 * Mirrors the `useMemoryController` / `useTasksBoardController` precedent.
 */
import { useCallback, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_cancel_side_channel,
  reflect_list_side_channels,
  reflect_start_side_channel,
  type ReflectSideChannelInfo,
  type ReflectStartSideChannelResult,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const SIDECHANNEL_QUERY_KEY = ['side-channels'] as const;
const SIDECHANNEL_STALE_MS = 5_000;

export interface SideChannelController {
  // Query
  channels: ReflectSideChannelInfo[];
  runningCount: number;
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Start form state
  showForm: boolean;
  toggleShowForm: () => void;
  agentName: string;
  setAgentName: (v: string) => void;
  prompt: string;
  setPrompt: (v: string) => void;

  // Mutations
  start: () => Promise<void>;
  cancel: (id: string) => Promise<void>;

  isMutating: boolean;
}

export function useSideChannelController(): SideChannelController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);

  const [showForm, setShowForm] = useState(false);
  const [agentName, setAgentName] = useState('default');
  const [prompt, setPrompt] = useState('');

  const channelsQ = useQuery({
    queryKey: [...SIDECHANNEL_QUERY_KEY],
    queryFn: () => reflect_list_side_channels(),
    staleTime: SIDECHANNEL_STALE_MS,
    refetchInterval: SIDECHANNEL_STALE_MS,
  });

  const invalidate = useCallback(
    () => qc.invalidateQueries({ queryKey: [...SIDECHANNEL_QUERY_KEY] }),
    [qc],
  );

  const startMut = useMutation({
    mutationFn: (args: { agent_name: string; prompt: string }) =>
      reflect_start_side_channel(args),
    onSuccess: (res: ReflectStartSideChannelResult) => {
      void invalidate();
      pushToast({
        kind: 'success',
        message: `Side-channel started: ${res.id.slice(0, 11)}…`,
      });
    },
  });

  const cancelMut = useMutation({
    mutationFn: (id: string) => reflect_cancel_side_channel(id),
    onSuccess: (ok: boolean, id: string) => {
      void invalidate();
      pushToast({
        kind: ok ? 'success' : 'warn',
        message: ok
          ? `Cancelled ${id.slice(0, 11)}…`
          : `${id.slice(0, 11)}… is already finished or unknown`,
      });
    },
  });

  const toggleShowForm = useCallback(() => setShowForm((v) => !v), []);

  const start = useCallback(async () => {
    const trimmedName = agentName.trim();
    const trimmedPrompt = prompt.trim();
    if (!trimmedName) {
      pushToast({ kind: 'warn', message: 'Agent name is required.' });
      return;
    }
    if (!trimmedPrompt) {
      pushToast({ kind: 'warn', message: 'Prompt is required.' });
      return;
    }
    try {
      await startMut.mutateAsync({ agent_name: trimmedName, prompt: trimmedPrompt });
      setPrompt('');
      setShowForm(false);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Start failed: ${(e as Error).message}`,
      });
    }
  }, [agentName, prompt, pushToast, startMut]);

  const cancel = useCallback(
    async (id: string) => {
      try {
        await cancelMut.mutateAsync(id);
      } catch (e) {
        pushToast({
          kind: 'error',
          message: `Cancel failed: ${(e as Error).message}`,
        });
      }
    },
    [cancelMut, pushToast],
  );

  const channels = channelsQ.data ?? [];
  const runningCount = channels.filter((c) => c.status === 'running').length;

  return {
    channels,
    runningCount,
    loading: channelsQ.isLoading,
    error: channelsQ.error,
    refetch: channelsQ.refetch,

    showForm,
    toggleShowForm,
    agentName,
    setAgentName,
    prompt,
    setPrompt,

    start,
    cancel,

    isMutating: startMut.isPending || cancelMut.isPending,
  };
}
