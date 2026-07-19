/**
 * Collaboration —— 阶段 5c:展示 reflect-agent 多 agent / 扩展能力状态。
 *
 * reflect-agent 支持 subagent / discussion / task / pipeline / goal 等多 agent
 * 编排(见 vendor/reflect-{subagent,discussion,task,pipeline,goal})。
 * 首期 GUI 未触发这些编排;此处展示已接入的扩展点(MCP server = 外部工具协作)
 * + 说明未来方向。不再是硬编码 STUB_USERS。
 */
import { useQuery } from '@tanstack/react-query';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_list_tools } from '@/utils/tauri';

export function CollaborationView() {
  const mcpServers = useAgentStore((s) => s.mcpServers);
  const lspServers = useAgentStore((s) => s.lspServers);
  const toolsQ = useQuery({ queryKey: ['tools'], queryFn: reflect_list_tools, staleTime: 30_000 });

  const mcpToolCount = (toolsQ.data ?? []).filter((t) => t.name.startsWith('mcp__')).length;

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Collaboration & Extensions</h1>

      <section style={{ marginBottom: 20 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>MCP servers (external tools)</h2>
        {mcpServers.length === 0 ? (
          <p style={{ fontSize: 13, color: '#888' }}>
            未配置 MCP server。在 Settings → Advanced 添加 <code>[mcp_servers.&lt;name&gt;]</code> 段。
          </p>
        ) : (
          <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
            {mcpServers.map((m) => (
              <li
                key={m.name}
                style={{
                  padding: '8px 12px',
                  border: '1px solid #e2e8f0',
                  borderRadius: 6,
                  marginBottom: 6,
                  fontSize: 13,
                }}
              >
                <span
                  style={{
                    display: 'inline-block',
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    background: m.status === 'started' ? '#22c55e' : '#ef4444',
                    marginRight: 8,
                  }}
                />
                <strong>{m.name}</strong>{' '}
                <span style={{ color: '#888' }}>
                  ({m.status}
                  {m.detail ? `: ${m.detail}` : ''})
                </span>
              </li>
            ))}
          </ul>
        )}
        {mcpToolCount > 0 && (
          <p style={{ fontSize: 12, color: '#666', marginTop: 8 }}>
            {mcpToolCount} 个 MCP 工具已注册(前缀 <code>mcp__</code>)。
          </p>
        )}
      </section>

      <section style={{ marginBottom: 20 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>LSP servers</h2>
        {lspServers.length === 0 ? (
          <p style={{ fontSize: 13, color: '#888' }}>未配置 LSP server。</p>
        ) : (
          <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
            {lspServers.map((l) => (
              <li key={l.name} style={{ fontSize: 13, marginBottom: 4 }}>
                <strong>{l.name}</strong> — {l.status}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section
        style={{
          padding: 12,
          background: '#f8fafc',
          borderRadius: 6,
          fontSize: 12,
          color: '#666',
        }}
      >
        <strong>多 agent 编排</strong>(subagent / discussion / task / pipeline / goal)
        是 reflect-agent 的核心能力,GUI 触发界面在后续阶段接入。当前可通过 Chat
        让 agent 自主调用这些能力(如"用子 agent 并行处理")。
      </section>
    </div>
  );
}
