/**
 * M3.x Apps —— App 集成。
 *
 * - 列出已连接的第三方应用
 * - 配置 app 级权限
 * - M3.x 扩展:VS Code / JetBrains / Cursor 集成
 */

import { useState } from 'react';

interface App {
  id: string;
  name: string;
  icon: string;
  connected: boolean;
  permissions: string[];
}

const STUB_APPS: App[] = [
  { id: 'vscode', name: 'VS Code', icon: '💻', connected: true, permissions: ['read_files', 'exec_command'] },
  { id: 'jetbrains', name: 'JetBrains', icon: '🧠', connected: false, permissions: [] },
  { id: 'cursor', name: 'Cursor', icon: '🎯', connected: false, permissions: [] },
];

export function AppsView() {
  const [apps, setApps] = useState<App[]>(STUB_APPS);

  const toggle = (id: string) => {
    setApps((prev) => prev.map((a) => (a.id === id ? { ...a, connected: !a.connected } : a)));
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Apps</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: 连接 IDE 和外部工具。当前为占位列表。
      </p>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {apps.map((a) => (
          <li
            key={a.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '12px 0',
              borderBottom: '1px solid #f1f5f9',
            }}
          >
            <span style={{ fontSize: 24 }}>{a.icon}</span>
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 500, fontSize: 14 }}>{a.name}</div>
              <div style={{ fontSize: 12, color: '#888' }}>
                {a.connected ? 'Connected' : 'Not connected'}
              </div>
            </div>
            <button
              onClick={() => toggle(a.id)}
              style={{
                padding: '4px 12px',
                border: '1px solid',
                borderColor: a.connected ? '#ef4444' : '#22c55e',
                background: a.connected ? '#fee2e2' : '#dcfce7',
                borderRadius: 4,
                cursor: 'pointer',
                fontSize: 12,
              }}
            >
              {a.connected ? 'Disconnect' : 'Connect'}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
