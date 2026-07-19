/**
 * Git —— 阶段 5b:移除 phantom invoke(reflect_git_status 不存在),
 * 改成"通过 agent 操作 git"——快捷按钮发预设 prompt。
 */
import { useState } from 'react';
import { useAgentStore } from '@/stores/agentStore';

const QUICK_PROMPTS = [
  { label: '查看当前分支和状态', prompt: '请用 bash 运行 `git status -sb` 并总结当前分支和工作区变更。' },
  { label: '查看最近提交', prompt: '请用 bash 运行 `git log --oneline -10` 展示最近 10 条提交。' },
  { label: '查看未推送的提交', prompt: '请用 bash 运行 `git log origin/HEAD..HEAD --oneline`(若失败说明无上游)。' },
  { label: '创建提交', prompt: '我想创建一个 git commit。请先用 git status 和 git diff 查看变更,然后建议 commit message。' },
];

export function GitView() {
  const submit = useAgentStore((s) => s.submit);
  const [sent, setSent] = useState<string | null>(null);

  const send = async (prompt: string) => {
    await submit(prompt);
    setSent(prompt);
    setTimeout(() => setSent(null), 2000);
  };

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Git</h1>
      <div style={{ padding: 12, marginBottom: 16, background: '#f8fafc', borderRadius: 6, fontSize: 13 }}>
        Git 操作通过 agent 的 <code>bash</code> 工具完成。点击快捷操作发到对话,
        或直接在 Chat 描述需求(如"帮我提交并推送")。
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
      </section>
    </div>
  );
}
