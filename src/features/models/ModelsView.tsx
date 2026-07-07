/**
 * M3.x Models —— 模型选择器。
 *
 * - 列出当前可用的 model spec
 * - 选择后通过 `reflect_set_effort` 下发
 * - M3.x 扩展:从 reflect-config 读真实 provider/model 列表
 */

import { useState } from 'react';
import { reflect_set_effort } from '@/utils/tauri';

const STUB_MODELS = [
  { id: 'stub/test', label: 'Stub Test (no network)', provider: 'local' },
  { id: 'anthropic/claude-sonnet-4-20250514', label: 'Claude Sonnet 4', provider: 'anthropic' },
  { id: 'openai/gpt-4o', label: 'GPT-4o', provider: 'openai' },
  { id: 'google/gemini-2.0-flash', label: 'Gemini 2.0 Flash', provider: 'google' },
];

const REASONING_EFFORTS = ['Low', 'Medium', 'High'] as const;

export function ModelsView() {
  const [selected, setSelected] = useState('stub/test');
  const [effort, setEffort] = useState<string>('Low');
  const [saved, setSaved] = useState(false);

  const onSave = async () => {
    await reflect_set_effort(effort);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Models</h1>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>Model</h2>
        <select
          value={selected}
          onChange={(e) => setSelected(e.target.value)}
          style={{ width: '100%', padding: 8, fontSize: 14 }}
        >
          {STUB_MODELS.map((m) => (
            <option key={m.id} value={m.id}>
              {m.label} ({m.provider})
            </option>
          ))}
        </select>
        <p style={{ fontSize: 11, color: '#888', marginTop: 4 }}>
          M3.x: 从 `~/.reflect/config.toml` 动态加载可用模型列表。
        </p>
      </section>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>Reasoning Effort</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          {REASONING_EFFORTS.map((e) => (
            <button
              key={e}
              onClick={() => setEffort(e)}
              style={{
                padding: '6px 14px',
                border: '1px solid',
                borderColor: effort === e ? '#3b82f6' : '#e2e8f0',
                background: effort === e ? '#dbeafe' : 'white',
                borderRadius: 6,
                cursor: 'pointer',
                fontSize: 13,
              }}
            >
              {e}
            </button>
          ))}
        </div>
        <button
          onClick={onSave}
          style={{
            marginTop: 12,
            padding: '8px 16px',
            background: '#3b82f6',
            color: 'white',
            border: 'none',
            borderRadius: 6,
            cursor: 'pointer',
            fontSize: 13,
          }}
        >
          {saved ? 'Saved ✓' : 'Save Effort'}
        </button>
      </section>
    </div>
  );
}
