/**
 * ThreadItem —— 单个 thread 行（CSS Modules 版）。
 * 用 router Link，保留 data-active 契约。
 *
 * B3-03: kebab 菜单(⋯) — rename / export / delete。
 */
import { useState, useRef, useEffect } from 'react';
import { Link } from '@tanstack/react-router';
import type { ReflectSessionInfo } from '@/utils/commands';
import { chatLinkFor, shortTimestamp } from '../utils/threadLabels';
import { ThreadItemMenu } from './ThreadItemMenu';
import s from './ThreadItem.module.css';

export interface ThreadItemProps {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
  onRename: (id: string, newName: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onExport: (id: string) => Promise<string | null>;
}

export function ThreadItem({
  session,
  active,
  onClick,
  onRename,
  onDelete,
  onExport,
}: ThreadItemProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const kebabRef = useRef<HTMLButtonElement | null>(null);

  // Close menu on outside click / Esc.
  useEffect(() => {
    if (!menuOpen) return;
    const onDocClick = (e: MouseEvent) => {
      const target = e.target as Node | null;
      if (kebabRef.current?.contains(target)) return;
      const menuEl = document.querySelector(`[data-thread-menu="${session.session_id}"]`);
      if (menuEl?.contains(target)) return;
      setMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenuOpen(false);
    };
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', onKey);
    };
  }, [menuOpen, session.session_id]);

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
          {session.session_id.slice(0, 8)}
        </div>
        <div className={s.meta}>
          {shortTimestamp(session.started_at)} · {session.message_count} msgs
        </div>
      </Link>
      <button
        ref={kebabRef}
        type="button"
        className={s.kebab}
        aria-label="Thread actions"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        data-testid={`thread-kebab-${session.session_id}`}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setMenuOpen((v) => !v);
        }}
      >
        ⋯
      </button>
      {menuOpen && (
        <ThreadItemMenu
          session={session}
          onRename={async (newName) => {
            await onRename(session.session_id, newName);
            setMenuOpen(false);
          }}
          onDelete={async () => {
            await onDelete(session.session_id);
            setMenuOpen(false);
          }}
          onExport={async () => {
            const path = await onExport(session.session_id);
            setMenuOpen(false);
            return path;
          }}
        />
      )}
    </li>
  );
}
