/**
 * useActivityController —— Activity timeline + @mention 搜索的
 * TanStack Query 控制器 (Phase 3 item 9)。
 *
 * 数据源:
 * - `reflect_list_activity` —— 全量 + 过滤(activity tab)。
 * - `reflect_search_activity` —— `@<query>` mention 搜索(mentions tab)。
 * - `reflect_clear_activity` —— 清空内存缓冲(诊断用)。
 */
import { useCallback, useMemo, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  reflect_list_activity,
  reflect_search_activity,
  reflect_clear_activity,
  type ReflectActivityEvent,
  type ReflectActivityFilter,
  type ReflectActivityLevel,
  extractMentions,
} from '@/utils/commands/activity';

export const ACTIVITY_QUERY_KEY = ['activity'] as const;
export const ACTIVITY_MENTIONS_QUERY_KEY = ['activity', 'mentions'] as const;
export const ACTIVITY_STALE_MS = 5_000;

export interface ActivityController {
  /** 顶部标签:inbox / activity / mentions。 */
  tab: 'inbox' | 'activity' | 'mentions';
  setTab: (t: ActivityController['tab']) => void;
  /** activity tab 的过滤(level:info/warn/error)。 */
  level: ReflectActivityLevel | 'all';
  setLevel: (l: ActivityController['level']) => void;
  /** mentions tab 的 @query 字符串。 */
  mentionQuery: string;
  setMentionQuery: (q: string) => void;
  /** 当前 tab 的事件数组(已是活动方向的最新优先)。 */
  events: ReflectActivityEvent[];
  /** 总数(诊断) */
  total: number;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
  clear: () => Promise<void>;
  /** 从纯文本提取 mention ID。 */
  extractMentions: (text: string) => string[];
}

export function useActivityController(): ActivityController {
  const qc = useQueryClient();
  const [tab, setTab] = useState<ActivityController['tab']>('inbox');
  const [level, setLevel] = useState<ActivityController['level']>('all');
  const [mentionQuery, setMentionQuery] = useState('');

  const activityFilter: ReflectActivityFilter | undefined = useMemo(() => {
    if (level === 'all') return { limit: 200 };
    return { level, limit: 200 };
  }, [level]);

  const activityQ = useQuery({
    queryKey: [...ACTIVITY_QUERY_KEY, 'list', activityFilter] as const,
    queryFn: () => reflect_list_activity(activityFilter),
    enabled: tab === 'activity',
    staleTime: ACTIVITY_STALE_MS,
  });

  const mentionsQ = useQuery({
    queryKey: [...ACTIVITY_MENTIONS_QUERY_KEY, mentionQuery] as const,
    queryFn: () => reflect_search_activity(mentionQuery),
    enabled: tab === 'mentions',
    staleTime: ACTIVITY_STALE_MS,
  });

  const clearMut = useMutation({
    mutationFn: () => reflect_clear_activity(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: [...ACTIVITY_QUERY_KEY] });
    },
  });

  const events =
    tab === 'activity'
      ? activityQ.data ?? []
      : tab === 'mentions'
      ? mentionsQ.data ?? []
      : [];
  const loading = tab === 'activity' ? activityQ.isLoading : tab === 'mentions' ? mentionsQ.isLoading : false;
  const error =
    tab === 'activity'
      ? activityQ.error
      : tab === 'mentions'
      ? mentionsQ.error
      : null;

  const refetch = useCallback(() => {
    if (tab === 'activity') void activityQ.refetch();
    else if (tab === 'mentions') void mentionsQ.refetch();
  }, [tab, activityQ, mentionsQ]);

  return {
    tab,
    setTab,
    level,
    setLevel,
    mentionQuery,
    setMentionQuery,
    events,
    total: events.length,
    loading,
    error: error as Error | null,
    refetch,
    clear: async () => {
      await clearMut.mutateAsync();
    },
    extractMentions,
  };
}
