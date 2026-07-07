/**
 * M3.x Settings —— 反射配置编辑。
 *
 * - 主题切换 (light / dark / system)
 * - 编辑器设置 (vim 模式)
 * - Provider 配置 (M3.x 写回 ~/.reflect/config.toml)
 * - 权限模式 (Auto / Ask / Deny)
 */

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reflect_set_permission_mode } from '@/utils/tauri';

type Theme = 'light' | 'dark' | 'system';
type PermissionMode = 'Auto' | 'Ask' | 'Deny';

export function SettingsView({ onClose }: { onClose?: () => void }) {
  const [theme, setTheme] = useState<Theme>('system');
  const [vim, setVim] = useState(false);
  const [provider, setProvider] = useState('anthropic');
  const [permission, setPermission] = useState<PermissionMode>('Auto');
  const [saved, setSaved] = useState(false);
  const qc = useQueryClient();

  const permMutation = useMutation({
    mutationFn: async (mode: string) => reflect_set_permission_mode(mode),
    onSuccess: () => {
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      qc.invalidateQueries({ queryKey: ['agent'] });
    },
  });

  const onSave = () => {
    permMutation.mutate(permission);
    // M3.x: persist theme/vim/provider to ~/.reflect/config.toml via Rust command
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div style={{ padding: 32, maxWidth: 640, margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 24 }}>
        <h1 style={{ fontSize: 22, margin: 0, flex: 1 }}>Settings</h1>
        {onClose && (
          <button onClick={onClose} style={{ padding: '4px 12px', cursor: 'pointer' }}>
            Close
          </button>
        )}
      </div>

      <Section title="Display">
        <label style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          Theme:
          <select
            value={theme}
            onChange={(e) => setTheme(e.target.value as Theme)}
            style={{ padding: 6, fontSize: 13 }}
          >
            <option value="light">Light</option>
            <option value="dark">Dark</option>
            <option value="system">System</option>
          </select>
        </label>
      </Section>

      <Section title="Editor">
        <label style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <input
            type="checkbox"
            checked={vim}
            onChange={(e) => setVim(e.target.checked)}
          />
          Enable vim keybindings
        </label>
      </Section>

      <Section title="Provider">
        <label style={{ display: 'block', marginBottom: 8 }}>
          Default provider:
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            style={{ marginLeft: 8, padding: 6, fontSize: 13 }}
          >
            <option value="anthropic">Anthropic</option>
            <option value="openai">OpenAI</option>
            <option value="google">Google</option>
            <option value="ollama">Ollama (local)</option>
          </select>
        </label>
      </Section>

      <Section title="Permissions">
        <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
          {(['Auto', 'Ask', 'Deny'] as PermissionMode[]).map((m) => (
            <button
              key={m}
              onClick={() => setPermission(m)}
              style={{
                padding: '6px 14px',
                border: '1px solid',
                borderColor: permission === m ? '#3b82f6' : '#e2e8f0',
                background: permission === m ? '#dbeafe' : 'white',
                borderRadius: 6,
                cursor: 'pointer',
                fontSize: 13,
              }}
            >
              {m}
            </button>
          ))}
        </div>
        <p style={{ fontSize: 11, color: '#888' }}>
          Auto: tools run without asking. Ask: prompt for approval. Deny: block all tools.
        </p>
      </Section>

      <button
        onClick={onSave}
        style={{
          padding: '10px 20px',
          background: '#3b82f6',
          color: 'white',
          border: 'none',
          borderRadius: 6,
          cursor: 'pointer',
          fontSize: 14,
        }}
      >
        {saved ? 'Saved ✓' : 'Save Settings'}
      </button>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section style={{ marginBottom: 24 }}>
      <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>{title}</h2>
      {children}
    </section>
  );
}
