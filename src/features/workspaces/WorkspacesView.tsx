/**
 * M3.x Workspaces —— 工作区管理。
 *
 * - 列出已知工作区 (当前 + 最近)
 * - 切换工作区
 * - M3.x 扩展:add / remove / open in terminal
 */

import { useState } from 'react';

interface Workspace {
  id: string;
  name: string;
  path: string;
  active: boolean;
}

const STUB_WORKSPACES: Workspace[] = [
  { id: 'current', name: 'Current', path: '.', active: true },
  { id: 'reflect-agent', name: 'Reflect-Agent', path: '../Reflect-Agent', active: false },
  { id: 'reflect-desktop', name: 'ReflectDesktop', path: '.', active: false },
];

export function WorkspacesView() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>(STUB_WORKSPACES);

  const switchTo = (id: string) => {
    setWorkspaces((prev) =>
      prev.map((w) => ({ ...w, active: w.id === id }))
    );
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Workspaces</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: 从 `~/.reflect/workspaces.toml` 加载。当前为占位列表。
      </p>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {workspaces.map((w) => (
          <li
            key={w.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '10px 12px',
              borderBottom: '1px solid #f1f5f9',
              background: w.active ? '#f0f9ff' : 'transparent',
              borderRadius: 6,
            }}
          >
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: w.active ? 600 : 400, fontSize: 14 }}>
                {w.name}
                {w.active && <span style={{ color: '#3b82f6', fontSize: 11, marginLeft: 8 }}>active</span>}
              </div>
              <div style={{ fontSize: 12, color: '#888' }}><code>{w.path}</code></div>
            </div>
            {!w.active && (
              <button
                onClick={() => switchTo(w.id)}
                style={{
                  padding: '4px 12px',
                  border: '1px solid #e2e8f0',
                  background: 'white',
                  borderRadius: 4,
                  cursor: 'pointer',
                  fontSize: 12,
                }}
              >
                Switch
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
