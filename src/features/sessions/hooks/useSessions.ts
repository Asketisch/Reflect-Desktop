/**
 * M3.x Session 列表 hook —— TanStack Query 驱动。
 *
 * - useQuery 缓存 `reflect_list_sessions` 结果(5min staleTime)
 * - useMutation 处理 rename → invalidate → 自动重刷
 * - 时间分桶规则已抽出到 `./utils/buckets.ts`(纯函数,可独立测试)
 *
 * CodexMonitor 同名: `src/features/threads/hooks/useThreads.ts`
 */
import { useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate, useLocation } from '@tanstack/react-router';
import {
  reflect_list_sessions,
  reflect_rename_session,
  reflect_delete_session,
  reflect_export_session,
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
  remove: (id: string) => Promise<void>;
  export: (id: string) => Promise<string | null>;
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

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => reflect_delete_session(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
    },
  });

  const rename = useCallback(async (id: string, newName: string) => {
    await renameMutation.mutateAsync({ id, newName });
  }, [renameMutation]);

  const remove = useCallback(async (id: string) => {
    await deleteMutation.mutateAsync(id);
  }, [deleteMutation]);

  const exportSession = useCallback(async (id: string) => {
    return reflect_export_session(id);
  }, []);

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
    remove,
    export: exportSession,
  };
}

/**
 * 路由驱动的活动 session id —— 与 ChatView / Sidebar 的 URL 同步。
 *
 * 读源: `/chat/$sessionId` 路由参数；fallback 到 `/chat`（无 id,新对话）。
 * 写入: `setActiveId(id)` 导航到 `/chat/$sessionId`,`setActiveId(null)` 导航到 `/chat`。
 *        `clear()` 显式清除(等价 null)。
 *
 * CodexMonitor 同名: `src/features/threads/hooks/useActiveThread.ts`
 */
export interface UseActiveSessionResult {
  activeId: string | null;
  setActiveId: (id: string | null) => void;
  clear: () => void;
}

export function useActiveSession(): UseActiveSessionResult {
  const navigate = useNavigate();
  const location = useLocation();

  // Derive sessionId from pathname: matches /chat/:sessionId but not /chat.
  const activeId = (() => {
    const m = location.pathname.match(/^\/chat\/([^/]+)$/);
    return m ? decodeURIComponent(m[1]) : null;
  })();

  const setActiveId = useCallback((id: string | null) => {
    if (id) {
      void navigate({ to: '/chat/$sessionId', params: { sessionId: id } });
    } else if (location.pathname.startsWith('/chat')) {
      void navigate({ to: '/chat' });
    }
  }, [navigate, location.pathname]);

  const clear = useCallback(() => {
    if (location.pathname.startsWith('/chat')) {
      void navigate({ to: '/chat' });
    }
  }, [navigate, location.pathname]);

  return { activeId, setActiveId, clear };
}
