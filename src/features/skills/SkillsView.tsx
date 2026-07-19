/**
 * Skills —— 阶段 5c:展示 agent 当前已注册的真实工具(来自 reflect_list_tools)。
 *
 * reflect-agent 的"能力"= ToolRegistry 里注册的工具(内置 + MCP plugin tool)。
 * 这里列出后端真实注册的工具及其 description,不再是硬编码 STUB_SKILLS。
 * 启用/禁用单个工具的 UI 留待后续(需后端 unregister 命令)。
 */
import { useQuery } from '@tanstack/react-query';
import { reflect_list_tools } from '@/utils/tauri';

export function SkillsView() {
  const toolsQ = useQuery({ queryKey: ['tools'], queryFn: reflect_list_tools, staleTime: 30_000 });

  const tools = toolsQ.data ?? [];
  const builtin = tools.filter((t) => !t.name.startsWith('mcp__'));
  const mcp = tools.filter((t) => t.name.startsWith('mcp__'));

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Skills & Tools</h1>
      <p style={{ color: '#666', fontSize: 13, marginBottom: 16 }}>
        Agent 当前可用的工具({tools.length} 个)。内置工具由 reflect-tools 提供,
        <code>mcp__</code> 前缀的工具来自 MCP server(在 Settings 配置)。
      </p>

      {toolsQ.isLoading && <p style={{ color: '#888' }}>Loading…</p>}
      {toolsQ.error && <p style={{ color: 'crimson' }}>Failed to load tools.</p>}

      <ToolGroup title={`内置工具 (${builtin.length})`} tools={builtin} />
      {mcp.length > 0 && <ToolGroup title={`MCP 工具 (${mcp.length})`} tools={mcp} />}
    </div>
  );
}

function ToolGroup({
  title,
  tools,
}: {
  title: string;
  tools: { name: string; description: string }[];
}) {
  return (
    <section style={{ marginBottom: 20 }}>
      <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>{title}</h2>
      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {tools.map((t) => (
          <li
            key={t.name}
            style={{
              display: 'flex',
              gap: 12,
              padding: '8px 0',
              borderBottom: '1px solid #f1f5f9',
            }}
          >
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 500, fontSize: 13 }}>
                <code>{t.name}</code>
              </div>
              <div style={{ fontSize: 12, color: '#888' }}>{t.description}</div>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
