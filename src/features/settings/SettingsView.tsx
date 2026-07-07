/**
 * M1.7 Settings —— Display/Editor/Provider 三段占位。 真正的三主题切换留到 M2。
 */
import { useState } from 'react';

interface Props {
  onClose: () => void;
}

export function SettingsView({ onClose }: Props) {
  const [theme, setTheme] = useState<'light' | 'dark' | 'system'>('system');
  const [vim, setVim] = useState(false);
  const [provider, setProvider] = useState('anthropic');

  return (
    <div style={{ padding: 24, maxWidth: 640 }}>
      <h1>Settings</h1>
      <p style={{ color: '#888' }}>M1.7 占位 — M2.x 写回 `~/.reflect/config.toml`</p>

      <section style={{ marginTop: 24 }}>
        <h2>Display</h2>
        <label style={{ display: 'block', marginBottom: 8 }}>
          Theme:
          <select
            value={theme}
            onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
            style={{ marginLeft: 8 }}
          >
            <option value="light">Light</option>
            <option value="dark">Dark</option>
            <option value="system">System</option>
          </select>
        </label>
      </section>

      <section style={{ marginTop: 24 }}>
        <h2>Editor</h2>
        <label>
          <input type="checkbox" checked={vim} onChange={(e) => setVim(e.target.checked)} />
          Enable vim keybindings
        </label>
      </section>

      <section style={{ marginTop: 24 }}>
        <h2>Provider</h2>
        <label style={{ display: 'block', marginBottom: 8 }}>
          Default provider:
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            style={{ marginLeft: 8 }}
          >
            <option value="anthropic">anthropic</option>
            <option value="openai">openai</option>
            <option value="mock">mock (dev only)</option>
          </select>
        </label>
        <small style={{ display: 'block', marginTop: 4, color: '#666' }}>
          API key 配置走 `~/.reflect/config.toml` ([providers.*].api_key).
        </small>
      </section>

      <div style={{ marginTop: 32, display: 'flex', gap: 8 }}>
        <button onClick={onClose}>Close</button>
        <button style={{ background: '#3b82f6', color: 'white', border: 'none', padding: '6px 14px' }}>
          Save (M2.x wiring)
        </button>
      </div>
    </div>
  );
}
