/**
 * M3.x Terminal —— 嵌入终端。
 *
 * - 简易终端 UI (输入框 + 输出历史)
 * - M3.x 扩展:真实 PTY 通过 Rust sidecar (`pty_process` crate)
 */

import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface TerminalLine {
  type: 'input' | 'output' | 'error';
  text: string;
}

export function TerminalView() {
  const [lines, setLines] = useState<TerminalLine[]>([
    { type: 'output', text: 'Reflect Desktop terminal — M3.x PTY integration coming soon.\n' },
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
    try {
      // M3.x: replace with real PTY command execution
      const result: string = await invoke('reflect_exec_command', { cmd });
      setLines((prev) => [...prev, { type: 'output', text: result + '\n' }]);
    } catch (e) {
      setLines((prev) => [...prev, { type: 'error', text: `error: ${e}\n` }]);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: '#1e1e1e', color: '#d4d4d4' }}>
      <div style={{ padding: '4px 12px', background: '#2d2d2d', fontSize: 12, borderBottom: '1px solid #3e3e3e' }}>
        Terminal
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: 12, fontFamily: 'monospace', fontSize: 13 }}>
        {lines.map((l, i) => (
          <div key={i} style={{ color: l.type === 'error' ? '#ef4444' : l.type === 'input' ? '#3b82f6' : '#d4d4d4' }}>
            {l.text}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
      <form onSubmit={(e) => { e.preventDefault(); run(); }} style={{ display: 'flex', padding: 8, borderTop: '1px solid #3e3e3e' }}>
        <span style={{ fontFamily: 'monospace', color: '#3b82f6', marginRight: 8 }}>$</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          style={{ flex: 1, background: 'transparent', border: 'none', color: '#d4d4d4', fontFamily: 'monospace', fontSize: 13, outline: 'none' }}
          placeholder="Enter command..."
        />
      </form>
    </div>
  );
}
