/**
 * ThreadItem —— 单个 thread 行(带 router Link)。
 */
import { Link } from '@tanstack/react-router';
import type { ReflectSessionInfo } from '@/utils/tauri';
import { chatLinkFor, shortTimestamp } from '../utils/threadLabels';

export interface ThreadItemProps {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
}

export function ThreadItem({ session, active, onClick }: ThreadItemProps) {
  return (
    <li style={{ marginBottom: 4 }}>
      <Link
        to="/chat/$sessionId"
        params={{ sessionId: session.session_id }}
        onClick={onClick}
        style={{
          display: 'block',
          padding: '10px 12px',
          background: active ? '#dbeafe' : 'transparent',
          border: '1px solid',
          borderColor: active ? '#93c5fd' : '#e2e8f0',
          borderRadius: 6,
          textDecoration: 'none',
          color: '#1e293b',
          fontSize: 14,
        }}
        data-active={active ? 'true' : 'false'}
        data-href={chatLinkFor(session)}
      >
        <div style={{ fontWeight: active ? 600 : 400 }}>
          {session.display_name || session.session_id}
        </div>
        <div style={{ fontSize: 11, color: '#888', marginTop: 2 }}>
          {shortTimestamp(session.started_at)} · {session.message_count} msgs
        </div>
      </Link>
    </li>
  );
}