/**
 * SessionItem —— 单个 session 行。
 *
 * CodexMonitor 同名: `src/features/threads/components/SessionItem.tsx`
 */
import { memo } from 'react';
import type { ReflectSessionInfo } from '@/utils/tauri';
import { displayTitle } from '../utils/buckets';

export interface SessionItemProps {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
}

function SessionItemImpl({ session, active, onClick }: SessionItemProps) {
  const title = displayTitle(session);
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      style={{
        display: 'block',
        width: '100%',
        textAlign: 'left',
        padding: '6px 8px',
        marginBottom: 4,
        border: active ? '1px solid #3b82f6' : '1px solid #eee',
        borderRadius: 4,
        background: active ? '#eef2ff' : 'white',
        cursor: 'pointer',
      }}
    >
      <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
      <div style={{ fontSize: 11, color: '#666' }}>
        {session.message_count} msgs · {session.token_total} tok
      </div>
    </button>
  );
}

export const SessionItem = memo(SessionItemImpl);