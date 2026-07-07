/**
 * M3.x Debug —— 调试面板 (dev only)。
 *
 * - 显示 backend state (AgentThread 状态)
 * - 显示 IPC 事件日志
 * - M3.x 扩展:webview inspector + network log
 */

import { useState, useEffect } from 'react';
import { useAgent } from '@/services/agent';

export function DebugView() {
  const { turns, session } = useAgent();
  const [events, setEvents] = useState<string[]>([]);

  useEffect(() => {
    setEvents((prev) => [...prev, `[${new Date().toLocaleTimeString()}] render`]);
  }, [turns.length, session]);

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Debug</h1>
      <p style={{ color: '#888', fontSize: 12 }}>Dev only — remove in production.</p>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 14, marginBottom: 8 }}>Agent State</h2>
        <pre style={{ background: '#f8fafc', padding: 12, borderRadius: 6, fontSize: 12, overflow: 'auto' }}>
          {JSON.stringify({ session, turns_count: turns.length }, null, 2)}
        </pre>
      </section>

      <section>
        <h2 style={{ fontSize: 14, marginBottom: 8 }}>Event Log</h2>
        <div style={{ background: '#1e1e1e', color: '#d4d4d4', padding: 12, borderRadius: 6, fontSize: 12, maxHeight: 300, overflow: 'auto' }}>
          {events.map((e, i) => (
            <div key={i}>{e}</div>
          ))}
        </div>
      </section>
    </div>
  );
}
