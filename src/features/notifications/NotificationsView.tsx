/**
 * Notifications —— 阶段 5c:从 store 派生真实运行时事件。
 *
 * 通知源:
 * - lastError(顶层错误 banner)
 * - pendingApprovals / pendingQuestions / pendingAskUser(待处理交互)
 * - mcpServers / lspServers(生命周期状态)
 *
 * 这些都是真实 agent 事件(store 已订阅 reflect_event),不再是硬编码 STUB。
 */
import { useAgentStore } from '@/stores/agentStore';

interface NotifItem {
  id: string;
  level: 'error' | 'warn' | 'info';
  title: string;
  body: string;
}

export function NotificationsView() {
  const lastError = useAgentStore((s) => s.lastError);
  const clearError = useAgentStore((s) => s.clearError);
  const approvals = useAgentStore((s) => s.pendingApprovals);
  const questions = useAgentStore((s) => s.pendingQuestions);
  const askUsers = useAgentStore((s) => s.pendingAskUser);
  const plan = useAgentStore((s) => s.pendingPlan);
  const mcpServers = useAgentStore((s) => s.mcpServers);
  const lspServers = useAgentStore((s) => s.lspServers);

  const items: NotifItem[] = [];
  if (lastError) {
    items.push({ id: 'err', level: 'error', title: 'Error', body: lastError });
  }
  for (const a of approvals) {
    items.push({
      id: `ap-${a.id}`,
      level: 'warn',
      title: `${a.kind} approval pending`,
      body: a.toolName ? `Tool: ${a.toolName}` : 'Awaiting your decision',
    });
  }
  for (const q of questions) {
    items.push({ id: `q-${q.id}`, level: 'warn', title: 'Question pending', body: 'Agent needs an answer' });
  }
  for (const u of askUsers) {
    items.push({ id: `u-${u.id}`, level: 'warn', title: 'Input pending', body: 'Agent needs your input' });
  }
  if (plan) {
    items.push({ id: 'plan', level: 'warn', title: 'Plan pending', body: 'Review and approve the plan' });
  }
  for (const m of mcpServers) {
    items.push({
      id: `mcp-${m.name}`,
      level: m.status === 'failed' ? 'error' : 'info',
      title: `MCP server ${m.name} ${m.status}`,
      body: m.detail ?? '',
    });
  }
  for (const l of lspServers) {
    items.push({
      id: `lsp-${l.name}`,
      level: l.status === 'failed' ? 'error' : 'info',
      title: `LSP server ${l.name} ${l.status}`,
      body: l.detail ?? '',
    });
  }

  const errorCount = items.filter((i) => i.level === 'error').length;

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16 }}>
        <h1 style={{ fontSize: 22, margin: 0, flex: 1 }}>Notifications</h1>
        {items.length > 0 && (
          <span
            style={{
              padding: '2px 8px',
              background: errorCount > 0 ? '#ef4444' : '#3b82f6',
              color: 'white',
              borderRadius: 12,
              fontSize: 11,
            }}
          >
            {items.length}
          </span>
        )}
        {lastError && (
          <button
            onClick={clearError}
            style={{ marginLeft: 12, padding: '4px 12px', cursor: 'pointer' }}
          >
            Dismiss error
          </button>
        )}
      </div>

      {items.length === 0 && <p style={{ color: '#888' }}>No notifications. Agent events will appear here.</p>}

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {items.map((n) => (
          <li
            key={n.id}
            style={{
              padding: 12,
              border: '1px solid #e2e8f0',
              borderLeft: `4px solid ${levelColor(n.level)}`,
              borderRadius: 8,
              marginBottom: 8,
              background: 'white',
            }}
          >
            <strong style={{ fontSize: 13 }}>{n.title}</strong>
            {n.body && (
              <p style={{ margin: '4px 0 0', fontSize: 13, color: '#666' }}>{n.body}</p>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

function levelColor(level: NotifItem['level']): string {
  switch (level) {
    case 'error':
      return '#ef4444';
    case 'warn':
      return '#f59e0b';
    default:
      return '#3b82f6';
  }
}
