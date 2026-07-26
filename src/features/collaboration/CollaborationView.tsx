/**
 * Collaboration & Extensions —— MCP/LSP server 状态展示（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { Network, Server, Cpu } from 'lucide-react';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_list_tools } from '@/utils/commands';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, EmptyState } from '@/features/design-system';
import s from './CollaborationView.module.css';

export function CollaborationView() {
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const toolsQ = useQuery({ queryKey: ['tools'], queryFn: reflect_list_tools, staleTime: 30_000 });

  const mcpToolCount = (toolsQ.data ?? []).filter((t) => t.name.startsWith('mcp__')).length;

  return (
    <PageShell
      icon={Network}
      title="Collaboration & Extensions"
      subtitle="External tools and language servers connected to the agent."
      width="md"
    >
      {/* MCP servers */}
      <section className={s.section}>
        <div className={s.sectionHeader}>
          <h3 className={s.sectionTitle}>
            <Icon icon={Server} size={14} />
            MCP servers
          </h3>
          {mcpServers.length > 0 && <Badge variant="neutral">{mcpServers.length}</Badge>}
        </div>
        {mcpServers.length === 0 ? (
          <Card level="flat" padding="none">
            <EmptyState
              size="sm"
              icon={<Icon icon={Server} />}
              title="No MCP servers configured"
              description={<>Add an <code className={s.codeInline}>[mcp_servers.&lt;name&gt;]</code> section in Settings → Advanced.</>}
            />
          </Card>
        ) : (
          <div className={s.serverList}>
            {mcpServers.map((m) => (
              <Card key={m.name} level="outlined" padding="sm" className={s.serverRow} data-status={m.status}>
                <span className={s.statusDot} data-status={m.status} />
                <div className={s.serverBody}>
                  <div className={s.serverName}>{m.name}</div>
                  {m.detail && <div className={s.serverDetail}>{m.detail}</div>}
                </div>
                <Badge variant={m.status === 'started' ? 'success' : 'danger'}>
                  {m.status}
                </Badge>
              </Card>
            ))}
          </div>
        )}
        {mcpToolCount > 0 && (
          <p className={s.hint}>{mcpToolCount} MCP tools registered (prefix <code className={s.codeInline}>mcp__</code>).</p>
        )}
      </section>

      {/* LSP servers */}
      <section className={s.section}>
        <div className={s.sectionHeader}>
          <h3 className={s.sectionTitle}>
            <Icon icon={Cpu} size={14} />
            LSP servers
          </h3>
          {lspServers.length > 0 && <Badge variant="neutral">{lspServers.length}</Badge>}
        </div>
        {lspServers.length === 0 ? (
          <Card level="flat" padding="none">
            <EmptyState size="sm" icon={<Icon icon={Cpu} />} title="No LSP servers configured" />
          </Card>
        ) : (
          <div className={s.serverList}>
            {lspServers.map((l) => (
              <Card key={l.name} level="outlined" padding="sm" className={s.serverRow} data-status={l.status}>
                <span className={s.statusDot} data-status={l.status} />
                <div className={s.serverBody}>
                  <div className={s.serverName}>{l.name}</div>
                  {l.detail && <div className={s.serverDetail}>{l.detail}</div>}
                </div>
                <Badge variant={l.status === 'started' ? 'success' : 'danger'}>{l.status}</Badge>
              </Card>
            ))}
          </div>
        )}
      </section>

      <Card level="outlined" padding="md" className={s.futureCard}>
        <div className={s.futureTitle}>Multi-agent orchestration</div>
        <p className={s.futureText}>
          Subagent / discussion / task / pipeline / goal orchestration is a core reflect-agent
          capability. GUI triggers will arrive in a later phase — for now, ask the agent in Chat
          (e.g. "use a subagent to parallelize").
        </p>
      </Card>
    </PageShell>
  );
}
