/**
 * M3.x Prompts —— Prompt 库 + 模板。
 *
 * - 列出内置 prompt 模板
 * - 点击插入到 Composer
 * - M3.x 扩展:custom prompts + folder organization + search
 */

import { useState } from 'react';

interface PromptTemplate {
  id: string;
  name: string;
  description: string;
  body: string;
}

const STUB_PROMPTS: PromptTemplate[] = [
  { id: 'p1', name: 'Review code', description: 'Review the current file for bugs and style', body: 'Review this code for bugs, security issues, and style violations.' },
  { id: 'p2', name: 'Write tests', description: 'Generate unit tests for the selected function', body: 'Write comprehensive unit tests for the following code.' },
  { id: 'p3', name: 'Explain', description: 'Explain the selected code in plain English', body: 'Explain what this code does in plain English.' },
  { id: 'p4', name: 'Refactor', description: 'Refactor the selected code for clarity', body: 'Refactor this code to be more readable and maintainable.' },
  { id: 'p5', name: 'Debug', description: 'Help debug an error', body: 'Help me debug the following error message and trace.' },
];

export function PromptsView() {
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Prompts</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: 从 `~/.reflect/prompts/` 目录加载。当前为占位列表。
      </p>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {STUB_PROMPTS.map((p) => (
          <li
            key={p.id}
            onClick={() => setSelected(p.id === selected ? null : p.id)}
            style={{
              padding: '12px',
              border: '1px solid #e2e8f0',
              borderRadius: 6,
              marginBottom: 8,
              cursor: 'pointer',
              background: p.id === selected ? '#f0f9ff' : 'white',
            }}
          >
            <div style={{ fontWeight: 500, fontSize: 14 }}>{p.name}</div>
            <div style={{ fontSize: 12, color: '#888', marginTop: 2 }}>{p.description}</div>
            {p.id === selected && (
              <pre
                style={{
                  marginTop: 8,
                  padding: 8,
                  background: '#f8fafc',
                  borderRadius: 4,
                  fontSize: 12,
                  whiteSpace: 'pre-wrap',
                }}
              >
                {p.body}
              </pre>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
