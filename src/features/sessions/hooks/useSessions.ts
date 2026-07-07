/**
 * M1.3 Session 列表 hook —— 调 `reflect_list_sessions` + 时间分桶。
 *
 * 时间分桶规则(沿用 CodexMonitor `Sidebar.tsx`):
 *   - Now       : < 1h
 *   - Today     : < 24h
 *   - Yesterday : 1-2 天前
 *   - This week : 3-7 天前
 *   - Older     : > 7 天
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
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
  const [all, setAll] = useState<ReflectSessionInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await reflect_list_sessions();
      setAll(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const buckets = useMemo<SessionBucket[]>(() => {
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
  }, [all]);

  const rename = useCallback(async (id: string, new_name: string) => {
    await reflect_rename_session(id, new_name);
    await refresh();
  }, [refresh]);

  return { buckets, all, loading, error, refresh, activeId, setActiveId, rename };
}
