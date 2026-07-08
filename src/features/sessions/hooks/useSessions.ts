/**
 * M3.x Session 列表 hook —— TanStack Query 驱动。
 *
 * - useQuery 缓存 `reflect_list_sessions` 结果(5min staleTime)
 * - useMutation 处理 rename → invalidate → 自动重刷
 * - 时间分桶规则已抽出到 `./utils/buckets.ts`(纯函数,可独立测试)
 *
 * CodexMonitor 同名: `src/features/threads/hooks/useThreads.ts`
 */
import { useState, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  reflect_list_sessions,
  reflect_rename_session,
  type ReflectSessionInfo,
} from '@/utils/tauri';
import { bucketSessions, type SessionBucket } from '../utils/buckets';

export type { SessionBucket };

export const SESSIONS_QUERY_KEY = ['sessions'] as const;
const SESSIONS_STALE_MS = 5 * 60_000;

export interface UseSessionsResult {
  buckets: SessionBucket[];
  all: ReflectSessionInfo[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
  refresh: () => void;
  rename: (id: string, newName: string) => Promise<void>;
}

export function useSessions(): UseSessionsResult {
  const qc = useQueryClient();
  const { data: all = [], isLoading, error, refetch } = useQuery({
    queryKey: SESSIONS_QUERY_KEY,
    queryFn: reflect_list_sessions,
    staleTime: SESSIONS_STALE_MS,
  });

  const buckets = bucketSessions(all);

  const renameMutation = useMutation({
    mutationFn: async ({ id, newName }: { id: string; newName: string }) =>
      reflect_rename_session(id, newName),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
    },
  });

  const rename = useCallback(async (id: string, newName: string) => {
    await renameMutation.mutateAsync({ id, newName });
  }, [renameMutation]);

  return {
    buckets,
    all,
    loading: isLoading,
    error: (error as Error | null)?.message ?? null,
    refetch: () => {
      void refetch();
    },
    refresh: () => {
      void refetch();
    },
    rename,
  };
}

/**
 * 维护当前活动 session id(local state,non-persisted)。
 *
 * CodexMonitor 同名: `src/features/threads/hooks/useActiveThread.ts`
 */
export function useActiveSession() {
  const [activeId, setActiveId] = useState<string | null>(null);
  return { activeId, setActiveId };
}