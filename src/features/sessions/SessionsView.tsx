/**
 * M1.3 SessionsView —— sidebar 顶层容器,合并 useSessions + useActiveSession。
 *
 * 之前 Sidebar.tsx 直接由 router 引用 —— 现在导出点统一到 SessionsView。
 */
import { useSessions, useActiveSession } from './hooks/useSessions';
import { Sidebar } from './components/Sidebar';

export function SessionsView() {
  const { buckets, loading, error, refresh } = useSessions();
  const { activeId, setActiveId } = useActiveSession();

  return (
    <Sidebar
      buckets={buckets}
      loading={loading}
      error={error}
      activeId={activeId}
      onSelect={setActiveId}
      onRefresh={refresh}
    />
  );
}