/**
 * M3.x Session 列表 hook —— TanStack Query 驱动。
 *
 * - useQuery 缓存 `reflect_list_sessions` 结果(5min staleTime)
 * - useMutation 处理 rename → invalidate → 自动重刷
 * - 时间分桶规则(沿用 CodexMonitor `Sidebar.tsx`):
 *   - Now       : < 1h
 *   - Today     : < 24h
 *   - Yesterday : 1-2 天前
 *   - This week : 3-7 天前
 *   - Older     : > 7 天
 */
import { useState, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  reflect_list_sessions,
  reflect_rename_session,
  type ReflectSessionInfo,
} from '@/utils/tauri';

export interface SessionBucket {
  label: string;
  sessions: ReflectSessionInfo[];
}

function bucket(s: ReflectSessionInfo, now: number): string {
  const t = Date.parse(s.started_at);
  if (Number.isNaN(t)) return 'Older';
  const ageMs = now - t;
  const h = ageMs / (1000 * 60 * 60);
  if (h < 1) return 'Now';
  if (h < 24) return 'Today';
  const d = h / 24;
  if (d < 2) return 'Yesterday';
  if (d < 7) return 'This week';
  return 'Older';
}

export function useSessions() {
  const qc = useQueryClient();
  const [activeId, setActiveId] = useState<string | null>(null);

  const { data: all = [], isLoading, error, refetch } = useQuery({
    queryKey: ['sessions'],
    queryFn: reflect_list_sessions,
    staleTime: 5 * 60_000,
  });

  const renameMutation = useMutation({
    mutationFn: async ({ id, newName }: { id: string; newName: string }) =>
      reflect_rename_session(id, newName),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['sessions'] });
    },
  });

  const buckets = all.length > 0 ? (() => {
    const now = Date.now();
    const groups = new Map<string, ReflectSessionInfo[]>();
    for (const s of all) {
      const k = bucket(s, now);
      if (!groups.has(k)) groups.set(k, []);
      groups.get(k)!.push(s);
    }
    const order = ['Now', 'Today', 'Yesterday', 'This week', 'Older'];
    return order
      .filter((l) => groups.has(l))
      .map((l) => ({ label: l, sessions: groups.get(l)! }));
  })() : [];

  const rename = useCallback(async (id: string, newName: string) => {
    await renameMutation.mutateAsync({ id, newName });
  }, [renameMutation]);

  return {
    buckets,
    all,
    loading: isLoading,
    error: (error as Error | null)?.message ?? null,
    refetch,
    refresh: () => refetch(),
    activeId,
    setActiveId,
    rename,
  };
}
