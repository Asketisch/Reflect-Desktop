/**
 * Prompts —— 阶段 5c:快捷 prompt 库。
 *
 * 点击模板直接发送到当前对话(经 store submit),比纯展示更实用。
 * 自定义 prompt(从 ~/.reflect/prompts/ 加载)留待后续后端扫描命令。
 */
import { useState } from 'react';
import { useAgentStore } from '@/stores/agentStore';

interface PromptTemplate {
  id: string;
  name: string;
  description: string;
  body: string;
}

const PROMPTS: PromptTemplate[] = [
  {
    id: 'review',
    name: 'Review code',
    description: '审查当前工作区的代码(找 bug / 安全 / 风格)',
    body: '请审查当前工作区的代码,关注:潜在的 bug、安全问题、风格一致性。先列出你审查的文件,再给出具体建议。',
  },
  {
    id: 'tests',
    name: 'Write tests',
    description: '为最近修改的代码补单元测试',
    body: '请先用 git diff 查看最近修改,然后为修改的函数补充单元测试。运行测试确认通过。',
  },
  {
    id: 'explain',
    name: 'Explain architecture',
    description: '解释当前项目的架构',
    body: '请阅读项目根目录的关键文件(README、Cargo.toml/package.json、入口),用简洁的中文解释这个项目的架构和模块职责。',
  },
  {
    id: 'refactor',
    name: 'Refactor',
    description: '重构指定代码以提升可读性',
    body: '我想重构代码以提升可读性和可维护性。请先问我具体想重构哪个文件或模块,然后给出重构方案。',
  },
  {
    id: 'debug',
    name: 'Debug error',
    description: '帮我调试一个错误',
    body: '我遇到了一个错误。请先问我错误信息和复现步骤,然后帮我定位根因并修复。',
  },
  {
    id: 'plan',
    name: 'Plan a feature',
    description: '进入 plan 模式规划一个新功能',
    body: '我想实现一个新功能。请进入 plan 模式,先了解需求,再给出实现计划供我审批。',
  },
];

export function PromptsView() {
  const submit = useAgentStore((s) => s.submit);
  const [selected, setSelected] = useState<string | null>(null);
  const [sent, setSent] = useState<string | null>(null);

  const send = async (p: PromptTemplate) => {
    await submit(p.body);
    setSent(p.id);
    setTimeout(() => setSent(null), 2000);
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Prompts</h1>
      <p style={{ color: '#666', fontSize: 13, marginBottom: 16 }}>
        点击模板发送到当前对话。自定义 prompt(从 <code>~/.reflect/prompts/</code> 加载)在后续阶段接入。
      </p>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {PROMPTS.map((p) => (
          <li
            key={p.id}
            style={{
              padding: 12,
              border: '1px solid #e2e8f0',
              borderRadius: 6,
              marginBottom: 8,
              background: selected === p.id ? '#f0f9ff' : 'white',
            }}
          >
            <div
              onClick={() => setSelected(p.id === selected ? null : p.id)}
              style={{ cursor: 'pointer' }}
            >
              <div style={{ fontWeight: 500, fontSize: 14 }}>{p.name}</div>
              <div style={{ fontSize: 12, color: '#888', marginTop: 2 }}>{p.description}</div>
            </div>
            {selected === p.id && (
              <>
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
                <button
                  onClick={() => send(p)}
                  style={{
                    marginTop: 8,
                    padding: '6px 14px',
                    background: sent === p.id ? '#22c55e' : '#3b82f6',
                    color: 'white',
                    border: 'none',
                    borderRadius: 4,
                    cursor: 'pointer',
                    fontSize: 12,
                  }}
                >
                  {sent === p.id ? '✓ 已发送 →' : '发送到对话'}
                </button>
              </>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
