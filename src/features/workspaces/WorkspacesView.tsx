/**
 * Workspaces —— 阶段 5c:从真实 sessions(rollout)派生工作区列表。
 *
 * 每个 session 记录了它运行的 cwd;聚合去重得到"最近用过的 workspace"列表。
 * 不再是硬编码 STUB_WORKSPACES。
 */
import { useQuery } from '@tanstack/react-query';
import { reflect_agent_status, reflect_list_sessions } from '@/utils/tauri';

interface WorkspaceEntry {
  path: string;
  sessionCount: number;
  lastUsed: string;
}

export function WorkspacesView() {
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });
  const sessionsQ = useQuery({
    queryKey: ['sessions'],
    queryFn: reflect_list_sessions,
    staleTime: 60_000,
  });

  // 聚合 sessions 按 cwd 去重。
  const workspaces: WorkspaceEntry[] = (() => {
    const map = new Map<string, WorkspaceEntry>();
    for (const s of sessionsQ.data ?? []) {
      const cwd = s.cwd || '(unknown)';
      const existing = map.get(cwd);
      if (existing) {
        existing.sessionCount += 1;
        if (s.started_at > existing.lastUsed) existing.lastUsed = s.started_at;
      } else {
        map.set(cwd, { path: cwd, sessionCount: 1, lastUsed: s.started_at });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.lastUsed.localeCompare(a.lastUsed));
  })();

  const currentWs = statusQ.data?.workspace;

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Workspaces</h1>

      <div
        style={{
          padding: 12,
          marginBottom: 16,
          background: '#f0f9ff',
          border: '1px solid #bae6fd',
          borderRadius: 6,
          fontSize: 13,
        }}
      >
        <strong>当前 workspace:</strong> <code>{currentWs ?? '(loading…)'}</code>
        <p style={{ margin: '6px 0 0', fontSize: 12, color: '#666' }}>
          切换 workspace 需在启动时设 cwd,或通过 agent 的 <code>EnterWorktree</code> /
          <code>ExitWorktree</code> 工具(后续阶段接 UI 触发)。
        </p>
      </div>

      <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>最近 workspace(按 session 聚合)</h2>
      {sessionsQ.isLoading ? (
        <p style={{ color: '#888' }}>Loading…</p>
      ) : workspaces.length === 0 ? (
        <p style={{ color: '#888' }}>暂无 session 记录。开始一次对话即会记录 workspace。</p>
      ) : (
        <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
          {workspaces.map((w) => (
            <li
              key={w.path}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 12,
                padding: '10px 12px',
                borderBottom: '1px solid #f1f5f9',
                background: w.path === currentWs ? '#f0f9ff' : 'transparent',
                borderRadius: 6,
              }}
            >
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: w.path === currentWs ? 600 : 400, fontSize: 14 }}>
                  {basename(w.path)}
                  {w.path === currentWs && (
                    <span style={{ color: '#3b82f6', fontSize: 11, marginLeft: 8 }}>active</span>
                  )}
                </div>
                <div style={{ fontSize: 12, color: '#888' }}>
                  <code>{w.path}</code> · {w.sessionCount} session(s) · {new Date(w.lastUsed).toLocaleDateString()}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function basename(p: string): string {
  const parts = p.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || p;
}
