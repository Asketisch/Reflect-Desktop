/**
 * SessionItem —— 单个 session 行（CSS Modules 版）。
 *
 * 契约：主行 button 元素 + aria-pressed（active 反馈）。
 * 传入任一操作 handler 时行尾显示悬停 kebab（⋯）菜单
 * （rename / export / archive / delete，见 SessionItemMenu）。
 */
import { memo, useState, useRef, useEffect } from 'react';
import type { ReflectSessionInfo } from '@/utils/commands';
import { useI18n } from '@/utils/i18n';
import { displayTitle } from '../utils/buckets';
import { SessionItemMenu } from './SessionItemMenu';
import s from './SessionItem.module.css';

export interface SessionItemProps {
  session: ReflectSessionInfo;
  active: boolean;
  onClick: () => void;
  onRename?: (id: string, newName: string) => Promise<void>;
  onDelete?: (id: string) => Promise<void>;
  onExport?: (id: string) => Promise<string | null>;
  onArchive?: (id: string) => Promise<void>;
}

function SessionItemImpl({
  session,
  active,
  onClick,
  onRename,
  onDelete,
  onExport,
  onArchive,
}: SessionItemProps) {
  const { t } = useI18n();
  const hasActions = Boolean(onRename && onDelete && onExport);
  const [menuOpen, setMenuOpen] = useState(false);
  const kebabRef = useRef<HTMLButtonElement | null>(null);

  // 外部点击 / Esc 时关闭菜单。
  useEffect(() => {
    if (!menuOpen) return;
    const onDocClick = (e: MouseEvent) => {
      const target = e.target as Node | null;
      if (kebabRef.current?.contains(target)) return;
      const menuEl = document.querySelector(`[data-session-menu="${session.session_id}"]`);
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

  const title = displayTitle(session);
  // workspace 归属:basename 显示在 meta 行,hover 显示全路径;
  // 旧会话(workspace 为 null)显示「未归属」。
  const wsLabel = session.workspace ? basename(session.workspace) : t('session.noWorkspace');
  return (
    <div className={s.row}>
      <button
        onClick={onClick}
        aria-pressed={active}
        className={s.item}
        data-active={active || undefined}
      >
        <div className={s.title}>{title || t('sidebar.untitled')}</div>
        <div className={s.meta}>
          <span className={s.workspace} title={session.workspace ?? undefined}>
            {wsLabel}
          </span>
          <span className={s.metaSep}>·</span>
          <span>{session.message_count} msgs</span>
        </div>
      </button>
      {hasActions && (
        <button
          ref={kebabRef}
          type="button"
          className={s.kebab}
          aria-label={t('threads.threadActions')}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          data-testid={`session-kebab-${session.session_id}`}
          onClick={() => setMenuOpen((v) => !v)}
        >
          ⋯
        </button>
      )}
      {menuOpen && hasActions && onRename && onDelete && onExport && (
        <SessionItemMenu
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
          onArchive={
            onArchive
              ? async () => {
                  await onArchive(session.session_id);
                  setMenuOpen(false);
                }
              : undefined
          }
        />
      )}
    </div>
  );
}

/** workspace 绝对路径 → 末段目录名(显示用;hover 仍展示全路径)。 */
function basename(p: string): string {
  const parts = p.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || p;
}

export const SessionItem = memo(SessionItemImpl);
