/**
 * M1.7 状态栏 —— 顶部 Topbar + 底部 BottomBar。
 *
 * 信息源: 当前 session model + provider(M2 加 token / context / cost / 工具计数)。
 * M1.x 仅 model/provider + 一个 theme 按钮。
 */
import { useAgent } from '@/services/agent';

export function Topbar() {
  const { session } = useAgent();
  return (
    <header
      style={{
        padding: '8px 16px',
        borderBottom: '1px solid #eee',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        fontSize: 12,
      }}
    >
      <strong style={{ fontSize: 14 }}>Reflect</strong>
      <span style={{ color: '#888' }}>M1.7 — status bar scaffold</span>
      <div style={{ flex: 1 }} />
      {session && (
        <span style={{ color: '#3b82f6', fontWeight: 500 }}>
          {session.model} @ {session.provider}
        </span>
      )}
    </header>
  );
}

export function BottomBar() {
  return (
    <footer
      style={{
        padding: '6px 16px',
        borderTop: '1px solid #eee',
        fontSize: 11,
        color: '#666',
        display: 'flex',
        gap: 16,
      }}
    >
      <span>theme: system</span>
      <span>vim: off</span>
      <span>plan: off</span>
      <span>permission: default</span>
      <div style={{ flex: 1 }} />
      <span>reflect-gui M1.7</span>
    </footer>
  );
}
