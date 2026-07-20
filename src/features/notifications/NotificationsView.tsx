/**
 * Notifications —— 运行时事件通知视图（CSS Modules 版）。
 *
 * 数据源：store 派生的真实事件（lastError / pending 队列 / MCP / LSP）。
 * 支持 All / Error / Pending 过滤。
 */
import { useMemo, useState } from 'react';
import { AlertCircle, AlertTriangle, Info, BellOff } from 'lucide-react';
import type { ComponentType } from 'react';
import { useAgentStore } from '@/stores/agentStore';
import { Card, Badge, Button, Icon, EmptyState } from '@/features/design-system';
import s from './NotificationsView.module.css';

interface NotifItem {
  id: string;
  level: 'error' | 'warn' | 'info';
  title: string;
  body: string;
}

type Filter = 'all' | 'error' | 'pending';

export function NotificationsView() {
  const lastError = useAgentStore((st) => st.lastError);
  const clearError = useAgentStore((st) => st.clearError);
  const approvals = useAgentStore((st) => st.pendingApprovals);
  const questions = useAgentStore((st) => st.pendingQuestions);
  const askUsers = useAgentStore((st) => st.pendingAskUser);
  const plan = useAgentStore((st) => st.pendingPlan);
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const [filter, setFilter] = useState<Filter>('all');

  const items: NotifItem[] = useMemo(() => {
    const out: NotifItem[] = [];
    if (lastError) {
      out.push({ id: 'err', level: 'error', title: 'Error', body: lastError });
    }
    for (const a of approvals) {
      out.push({
        id: `ap-${a.id}`,
        level: 'warn',
        title: `${a.kind} approval pending`,
        body: a.toolName ? `Tool: ${a.toolName}` : 'Awaiting your decision',
      });
    }
    for (const q of questions) {
      out.push({ id: `q-${q.id}`, level: 'warn', title: 'Question pending', body: 'Agent needs an answer' });
    }
    for (const u of askUsers) {
      out.push({ id: `u-${u.id}`, level: 'warn', title: 'Input pending', body: 'Agent needs your input' });
    }
    if (plan) {
      out.push({ id: 'plan', level: 'warn', title: 'Plan pending', body: 'Review and approve the plan' });
    }
    for (const m of mcpServers) {
      out.push({
        id: `mcp-${m.name}`,
        level: m.status === 'failed' ? 'error' : 'info',
        title: `MCP server ${m.name} ${m.status}`,
        body: m.detail ?? '',
      });
    }
    for (const l of lspServers) {
      out.push({
        id: `lsp-${l.name}`,
        level: l.status === 'failed' ? 'error' : 'info',
        title: `LSP server ${l.name} ${l.status}`,
        body: l.detail ?? '',
      });
    }
    return out;
  }, [lastError, approvals, questions, askUsers, plan, mcpServers, lspServers]);

  const errorCount = items.filter((i) => i.level === 'error').length;
  const pendingCount = items.filter((i) => i.level === 'warn').length;

  const filtered = items.filter((i) => {
    if (filter === 'error') return i.level === 'error';
    if (filter === 'pending') return i.level === 'warn';
    return true;
  });

  return (
    <div className={s.root}>
      <header className={s.header}>
        <h1 className={s.title}>Notifications</h1>
        {items.length > 0 && (
          <div className={s.countBadges}>
            {errorCount > 0 && <Badge variant="danger" solid>{errorCount} error</Badge>}
            {pendingCount > 0 && <Badge variant="warning" solid>{pendingCount} pending</Badge>}
            {errorCount === 0 && pendingCount === 0 && (
              <Badge variant="neutral">{items.length}</Badge>
            )}
          </div>
        )}
      </header>

      <div className={s.filters}>
        <FilterBtn active={filter === 'all'} onClick={() => setFilter('all')}>
          All <span className={s.count}>{items.length}</span>
        </FilterBtn>
        <FilterBtn active={filter === 'error'} onClick={() => setFilter('error')}>
          Errors <span className={s.count}>{errorCount}</span>
        </FilterBtn>
        <FilterBtn active={filter === 'pending'} onClick={() => setFilter('pending')}>
          Pending <span className={s.count}>{pendingCount}</span>
        </FilterBtn>
        {lastError && (
          <Button variant="ghost" size="sm" onClick={clearError} className={s.dismissBtn}>
            Dismiss error
          </Button>
        )}
      </div>

      {filtered.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={BellOff} />}
            title="No notifications"
            description="Agent events, pending approvals, and server status will appear here."
          />
        </Card>
      ) : (
        <div className={s.list}>
          {filtered.map((n) => (
            <NotifRow key={n.id} item={n} />
          ))}
        </div>
      )}
    </div>
  );
}

function FilterBtn({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button className={s.filterBtn} data-active={active || undefined} onClick={onClick}>
      {children}
    </button>
  );
}

const LEVEL_ICON: Record<NotifItem['level'], ComponentType> = {
  error: AlertCircle,
  warn: AlertTriangle,
  info: Info,
};

function NotifRow({ item }: { item: NotifItem }) {
  const IconComp = LEVEL_ICON[item.level];
  return (
    <Card level="outlined" padding="md" className={s.notif} data-level={item.level}>
      <div className={s.notifIcon}>
        <Icon icon={IconComp} size={16} />
      </div>
      <div className={s.notifBody}>
        <div className={s.notifTitle}>{item.title}</div>
        {item.body && <div className={s.notifBodyText}>{item.body}</div>}
      </div>
    </Card>
  );
}
