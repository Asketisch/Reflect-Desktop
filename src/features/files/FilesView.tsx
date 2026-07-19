/**
 * Files —— 阶段 5b:移除 phantom invoke(reflect_list_files 不存在),
 * 改成"通过 agent 探索文件"——快捷按钮发预设 prompt 给 agent。
 *
 * 真正的文件树 UI 需要后端 list_files 命令(避免与 agent 的 read/glob 工具
 * 重复,首期不新建并行命令面,统一经 agent 对话)。
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_agent_status } from '@/utils/tauri';

const QUICK_PROMPTS = [
  { label: '列出当前目录文件', prompt: '请用 glob 工具列出当前工作区根目录的文件和子目录。' },
  { label: '查找最近修改的文件', prompt: '请找出工作区内最近修改过的 10 个文件(用 bash: git log 或 find -mtime)。' },
  { label: '搜索文件名', prompt: '我想按文件名搜索,请告诉我你用 glob 工具需要什么 pattern。' },
];

export function FilesView() {
  const submit = useAgentStore((s) => s.submit);
  const [sent, setSent] = useState<string | null>(null);
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  const send = async (prompt: string) => {
    await submit(prompt);
    setSent(prompt);
    setTimeout(() => setSent(null), 2000);
  };

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Files</h1>

      <div
        style={{
          padding: 12,
          marginBottom: 16,
          background: '#f8fafc',
          borderRadius: 6,
          fontSize: 13,
        }}
      >
        <strong>Workspace:</strong>{' '}
        <code>{statusQ.data?.workspace ?? '(loading…)'}</code>
        <p style={{ margin: '8px 0 0', fontSize: 12, color: '#666' }}>
          文件浏览通过 agent 的 <code>read</code> / <code>glob</code> / <code>grep</code> 工具完成。
          点击下面的快捷操作发到对话,或直接在 Chat 里描述需求。
        </p>
      </div>

      <section>
        <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>快捷操作</h2>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {QUICK_PROMPTS.map((q) => (
            <button
              key={q.label}
              onClick={() => send(q.prompt)}
              style={{
                padding: '10px 14px',
                textAlign: 'left',
                border: '1px solid #e2e8f0',
                borderRadius: 6,
                background: sent === q.prompt ? '#dcfce7' : 'white',
                cursor: 'pointer',
                fontSize: 13,
              }}
            >
              {sent === q.prompt ? '✓ 已发送到对话 → ' : ''}
              {q.label}
            </button>
          ))}
        </div>
        <p style={{ fontSize: 11, color: '#888', marginTop: 12 }}>
          发送后切换到 Chat 查看结果。
        </p>
      </section>
    </div>
  );
}
