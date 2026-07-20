/**
 * Terminal —— 命令经 agent bash 工具执行（CSS Modules 版）。
 *
 * 真实 PTY 需要 Rust sidecar，首期不引入。
 */
import { useState, useRef, useEffect } from 'react';
import { Terminal as TerminalIcon, CornerDownLeft } from 'lucide-react';
import { useAgentStore } from '@/stores/agentStore';
import { Icon } from '@/features/design-system';
import s from './TerminalView.module.css';

interface TerminalLine {
  type: 'input' | 'info';
  text: string;
}

export function TerminalView() {
  const submit = useAgentStore((st) => st.submit);
  const [lines, setLines] = useState<TerminalLine[]>([
    {
      type: 'info',
      text: 'Commands are executed via the agent bash tool. Output appears in Chat.\n',
    },
  ]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  const run = async () => {
    const cmd = input.trim();
    if (!cmd) return;
    setBusy(true);
    setLines((prev) => [...prev, { type: 'input', text: `$ ${cmd}\n` }]);
    setInput('');
    try {
      await submit(`请用 bash 工具运行以下命令并展示输出:\n\n\`\`\`\n${cmd}\n\`\`\``);
      setLines((prev) => [...prev, { type: 'info', text: '→ sent to agent. Switch to Chat for output.\n' }]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={s.root}>
      <div className={s.header}>
        <Icon icon={TerminalIcon} size={14} />
        <span>Terminal (via agent bash)</span>
      </div>
      <div className={s.screen}>
        {lines.map((l, i) => (
          <div key={i} className={s.line} data-type={l.type}>
            {l.text}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
      <form
        className={s.form}
        onSubmit={(e) => {
          e.preventDefault();
          if (!busy) void run();
        }}
      >
        <span className={s.prompt}>$</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          className={s.input}
          placeholder="Enter command (executed via agent bash tool)..."
          autoFocus
          disabled={busy}
        />
        <Icon icon={CornerDownLeft} size={12} className={s.enterHint} />
      </form>
    </div>
  );
}
