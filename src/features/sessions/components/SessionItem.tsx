/**
 * SessionItem —— 单个 session 行（CSS Modules 版）。
 *
 * 契约：button 元素 + aria-pressed（active 反馈）。
 */
import { memo } from 'react';
import type { ReflectSessionInfo } from '@/utils/tauri';
import { displayTitle } from '../utils/buckets';
import s from './SessionItem.module.css';

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
      className={s.item}
      data-active={active || undefined}
    >
      <div className={s.title}>{title}</div>
      <div className={s.meta}>
        {session.message_count} msgs · {session.token_total} tok
      </div>
    </button>
  );
}

export const SessionItem = memo(SessionItemImpl);
