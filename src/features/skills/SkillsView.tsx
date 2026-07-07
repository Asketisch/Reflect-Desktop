/**
 * M3.x Skills —— 技能目录管理。
 *
 * - 列出已安装的 Reflect skills
 * - 启用 / 禁用切换
 * - M3.x 扩展:install from registry + 参数配置
 */

import { useState } from 'react';

interface Skill {
  name: string;
  description: string;
  enabled: boolean;
}

const STUB_SKILLS: Skill[] = [
  { name: 'bash', description: 'Execute shell commands in workspace', enabled: true },
  { name: 'read', description: 'Read file contents', enabled: true },
  { name: 'write', description: 'Write file contents', enabled: true },
  { name: 'edit', description: 'Edit file contents', enabled: true },
  { name: 'grep', description: 'Search file contents', enabled: true },
  { name: 'glob', description: 'Match file paths by pattern', enabled: true },
  { name: 'web_fetch', description: 'Fetch URL contents', enabled: true },
  { name: 'ask_user', description: 'Prompt user for input', enabled: true },
  { name: 'context_remaining', description: 'Report remaining context budget', enabled: true },
  { name: 'tool_search', description: 'Search available tools', enabled: true },
];

export function SkillsView() {
  const [skills, setSkills] = useState<Skill[]>(STUB_SKILLS);

  const toggle = (name: string) => {
    setSkills((prev) =>
      prev.map((s) => (s.name === name ? { ...s, enabled: !s.enabled } : s))
    );
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Skills</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: 从 `~/.reflect/skills/` 目录动态加载。当前为占位列表。
      </p>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {skills.map((s) => (
          <li
            key={s.name}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '10px 0',
              borderBottom: '1px solid #f1f5f9',
            }}
          >
            <input
              type="checkbox"
              checked={s.enabled}
              onChange={() => toggle(s.name)}
            />
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 500, fontSize: 14 }}>{s.name}</div>
              <div style={{ fontSize: 12, color: '#888' }}>{s.description}</div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
