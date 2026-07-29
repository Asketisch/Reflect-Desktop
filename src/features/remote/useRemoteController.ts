/**
 * Remote controller hook (Phase 2 item 2).
 *
 * Owns TanStack queries + mutations:
 *   - `reflect_get_remote_config` / `reflect_update_remote_config`
 *   - `reflect_get_remote_status`
 *   - `reflect_tailscale_status`
 *   - `reflect_tailscale_daemon_command_preview`
 *
 * The `start/stop/status` daemon commands are present so the iOS setup
 * card can surface their "not implemented yet" placeholders without a
 * TypeScript error; the controller does not call them automatically.
 */
import { useCallback, useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_get_remote_config,
  reflect_get_remote_status,
  reflect_tailscale_daemon_command_preview,
  reflect_tailscale_status,
  reflect_update_remote_config,
  type ReflectRemoteConfigSnapshot,
  type ReflectRemoteStatus,
  type ReflectTailscaleStatus,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';

export const REMOTE_QUERY_KEY = ['remote'] as const;
const REMOTE_STALE_MS = 15_000;

export interface RemoteController {
  // Query
  config: ReflectRemoteConfigSnapshot | null;
  status: ReflectRemoteStatus | null;
  tailscale: ReflectTailscaleStatus | null;
  previewCmd: string | null;
  loading: boolean;
  error: unknown;
  refetch: () => void;

  // Editor state
  showForm: boolean;
  toggleShowForm: () => void;
  draft: RemoteConfigDraft;
  patchDraft: (patch: Partial<RemoteConfigDraft>) => void;

  // Mutations
  save: () => Promise<void>;
  resetForm: () => void;

  isMutating: boolean;
}

export interface RemoteConfigDraft {
  host: string;
  port: number;
  authToken: string;
  autoConnect: boolean;
}

export function emptyRemoteDraft(c?: ReflectRemoteConfigSnapshot | null): RemoteConfigDraft {
  return {
    host: c?.host ?? '',
    port: c?.port ?? 4732,
    authToken: c?.auth_token ?? '',
    autoConnect: c?.auto_connect ?? false,
  };
}

export function useRemoteController(): RemoteController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);

  const [showForm, setShowForm] = useState(false);
  const [draft, setDraft] = useState<RemoteConfigDraft>(emptyRemoteDraft());

  const cfgQ = useQuery({
    queryKey: [...REMOTE_QUERY_KEY, 'config'],
    queryFn: () => reflect_get_remote_config(),
    staleTime: REMOTE_STALE_MS,
  });

  const statusQ = useQuery({
    queryKey: [...REMOTE_QUERY_KEY, 'status'],
    queryFn: () => reflect_get_remote_status(),
    staleTime: REMOTE_STALE_MS,
  });

  const tsQ = useQuery({
    queryKey: [...REMOTE_QUERY_KEY, 'tailscale'],
    queryFn: () => reflect_tailscale_status(),
    staleTime: REMOTE_STALE_MS,
  });

  const previewQ = useQuery({
    queryKey: [...REMOTE_QUERY_KEY, 'preview'],
    queryFn: () => reflect_tailscale_daemon_command_preview(),
    staleTime: Infinity,
  });

  const updateMut = useMutation({
    mutationFn: (d: RemoteConfigDraft) =>
      reflect_update_remote_config({
        host: d.host,
        port: d.port,
        auth_token: d.authToken || null,
        auto_connect: d.autoConnect,
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: [...REMOTE_QUERY_KEY] }),
  });

  const toggleShowForm = useCallback(() => setShowForm((v) => !v), []);

  // When toggling the form open, hydrate the draft from the current config.
  useEffect(() => {
    if (showForm) {
      setDraft(emptyRemoteDraft(cfgQ.data));
    }
  }, [showForm, cfgQ.data]);

  const patchDraft = useCallback(
    (patch: Partial<RemoteConfigDraft>) =>
      setDraft((prev) => ({ ...prev, ...patch })),
    [],
  );

  const save = useCallback(async () => {
    try {
      await updateMut.mutateAsync(draft);
      pushToast({ kind: 'success', message: 'Remote config saved.' });
      setShowForm(false);
    } catch (e) {
      pushToast({
        kind: 'error',
        message: `Save failed: ${(e as Error).message}`,
      });
    }
  }, [draft, pushToast, updateMut]);

  const resetForm = useCallback(() => {
    setDraft(emptyRemoteDraft(cfgQ.data));
  }, [cfgQ.data]);

  return {
    config: cfgQ.data ?? null,
    status: statusQ.data ?? null,
    tailscale: tsQ.data ?? null,
    previewCmd: previewQ.data ?? null,
    loading: cfgQ.isLoading || tsQ.isLoading,
    error: cfgQ.error,
    refetch: () => {
      void cfgQ.refetch();
      void statusQ.refetch();
      void tsQ.refetch();
    },

    showForm,
    toggleShowForm,
    draft,
    patchDraft,

    save,
    resetForm,

    isMutating: updateMut.isPending,
  };
}