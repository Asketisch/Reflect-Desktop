/**
 * ThreadItem —— 单个 thread 行（CSS Modules 版）。
 * 用 router Link，保留 data-active 契约。
 */
import { Link } from '@tanstack/react-router';
import type { ReflectSessionInfo } from '@/utils/tauri';
import { chatLinkFor, shortTimestamp } from '../utils/threadLabels';
import s from './ThreadItem.module.css';

export interface ThreadItemProps {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
}

export function ThreadItem({ session, active, onClick }: ThreadItemProps) {
  return (
    <li className={s.row}>
      <Link
        to="/chat/$sessionId"
        params={{ sessionId: session.session_id }}
        onClick={onClick}
        className={s.link}
        data-active={active ? 'true' : 'false'}
        data-href={chatLinkFor(session)}
      >
        <div className={s.title}>
          {session.display_name || session.session_id}
        </div>
        <div className={s.meta}>
          {shortTimestamp(session.started_at)} · {session.message_count} msgs
        </div>
      </Link>
    </li>
  );
}
