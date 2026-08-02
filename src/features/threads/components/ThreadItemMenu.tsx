/**
 * ThreadItemMenu —— ThreadItem 的 kebab 下拉菜单(rename / export / delete)。
 *
 * B3-03: 三动作 —— rename 用 prompt() 轻量处理(避免引入新 modal 原语);
 *                delete 二次 confirm; export 调 reflect_export_session。
 */
import { useState } from 'react';
import type { ReflectSessionInfo } from '@/utils/commands';
import { useI18n } from '@/utils/i18n';
import s from './ThreadItemMenu.module.css';

export interface ThreadItemMenuProps {
  session: ReflectSessionInfo;
  onRename: (newName: string) => Promise<void>;
  onDelete: () => Promise<void>;
  onExport: () => Promise<string | null>;
}

export function ThreadItemMenu({ session, onRename, onDelete, onExport }: ThreadItemMenuProps) {
  const { t } = useI18n();
  const [renameOpen, setRenameOpen] = useState(false);
  // `ReflectSessionInfo` (backend `SessionInfo`) only carries `session_id` —
  // there is no `display_name` field, so the rename input starts empty.
  const [renameValue, setRenameValue] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);

  const handleRename = async () => {
    const trimmed = renameValue.trim();
    if (!trimmed) {
      setRenameOpen(false);
      return;
    }
    setPending(true);
    setError(null);
    try {
      await onRename(trimmed);
      setRenameOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm(t('threads.deleteConfirm', { id: session.session_id }))) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      await onDelete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };

  const handleExport = async () => {
    setPending(true);
    setError(null);
    try {
      const path = await onExport();
      setExportPath(path);
      setTimeout(() => setExportPath(null), 3000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };

  if (renameOpen) {
    return (
      <div className={s.menu} data-thread-menu={session.session_id} role="dialog" aria-label={t('threads.renameThread')}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void handleRename();
          }}
        >
          <input
            type="text"
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            placeholder={t('threads.threadName')}
            disabled={pending}
            autoFocus
            className={s.input}
            data-testid={`thread-rename-input-${session.session_id}`}
          />
          <div className={s.actions}>
            <button type="button" onClick={() => setRenameOpen(false)} disabled={pending}>
              {t('threads.cancel')}
            </button>
            <button type="submit" disabled={pending || !renameValue.trim()}>
              {pending ? t('threads.saving') : t('threads.save')}
            </button>
          </div>
        </form>
        {error && <div className={s.error}>{error}</div>}
      </div>
    );
  }

  return (
    <div className={s.menu} data-thread-menu={session.session_id} role="menu" aria-label={t('threads.threadActions')}>
      <button
        type="button"
        role="menuitem"
        className={s.item}
        onClick={() => setRenameOpen(true)}
        disabled={pending}
        data-testid={`thread-rename-${session.session_id}`}
      >
        ✎ {t('threads.rename')}
      </button>
      <button
        type="button"
        role="menuitem"
        className={s.item}
        onClick={() => void handleExport()}
        disabled={pending}
        data-testid={`thread-export-${session.session_id}`}
      >
        ↓ {t('threads.export')}
      </button>
      <button
        type="button"
        role="menuitem"
        className={`${s.item} ${s.danger}`}
        onClick={() => void handleDelete()}
        disabled={pending}
        data-testid={`thread-delete-${session.session_id}`}
      >
        🗑 {t('threads.delete')}
      </button>
      {exportPath && (
        <div className={s.toast} role="status">
          {t('threads.exported', { path: exportPath })}
        </div>
      )}
      {error && <div className={s.error}>{error}</div>}
    </div>
  );
}