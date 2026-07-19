/**
 * Terminal —— 阶段 5b:移除 phantom invoke(reflect_exec_command 不存在)。
 *
 * 保留终端 UI,但命令经 agent 的 bash 工具执行(在 Chat 看输出)。
 * 真实 PTY 需要 Rust sidecar,首期不引入(避免与 agent bash 工具重复)。
 */
import { useState, useRef, useEffect } from 'react';
import { useAgentStore } from '@/stores/agentStore';

interface TerminalLine {
  type: 'input' | 'info';
  text: string;
}

export function TerminalView() {
  const submit = useAgentStore((s) => s.submit);
  const [lines, setLines] = useState<TerminalLine[]>([
    {
      type: 'info',
      text: '命令通过 agent 的 bash 工具执行,输出在 Chat 面板查看。\n',
    },
  ]);
  const [input, setInput] = useState('');
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  const run = async () => {
    const cmd = input.trim();
    if (!cmd) return;
    setLines((prev) => [...prev, { type: 'input', text: `$ ${cmd}\n` }]);
    setInput('');
    // 发到 agent —— agent 会用 bash 工具执行并在 Chat 输出结果。
    await submit(`请用 bash 工具运行以下命令并展示输出:\n\n\`\`\`\n${cmd}\n\`\`\``);
    setLines((prev) => [...prev, { type: 'info', text: '→ 已发送,切换到 Chat 查看输出\n' }]);
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: '#1e1e1e',
        color: '#d4d4d4',
      }}
    >
      <div
        style={{
          padding: '4px 12px',
          background: '#2d2d2d',
          fontSize: 12,
          borderBottom: '1px solid #3e3e3e',
        }}
      >
        Terminal (via agent bash)
      </div>
      <div
        style={{
          flex: 1,
          overflow: 'auto',
          padding: 12,
          fontFamily: 'monospace',
          fontSize: 13,
        }}
      >
        {lines.map((l, i) => (
          <div
            key={i}
            style={{ color: l.type === 'input' ? '#3b82f6' : '#888' }}
          >
            {l.text}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          run();
        }}
        style={{ display: 'flex', padding: 8, borderTop: '1px solid #3e3e3e' }}
      >
        <span style={{ fontFamily: 'monospace', color: '#3b82f6', marginRight: 8 }}>$</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          style={{
            flex: 1,
            background: 'transparent',
            border: 'none',
            color: '#d4d4d4',
            fontFamily: 'monospace',
            fontSize: 13,
            outline: 'none',
          }}
          placeholder="Enter command (executed via agent bash tool)..."
        />
      </form>
    </div>
  );
}
