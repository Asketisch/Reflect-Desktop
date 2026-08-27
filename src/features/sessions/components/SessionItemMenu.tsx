/**
 * SessionItemMenu —— session 行的操作下拉菜单（rename / export / archive / delete）。
 *
 * 由 Sidebar（主壳历史列表）与 ThreadsView（/sessions 全量视图）共用；
 * `onArchive` 可选（调用方未接归档能力时不渲染该菜单项）。
 *
 * - rename 用内联表单轻量处理（避免引入新 modal 原语）；
 * - archive / delete 二次 confirm；
 * - export 调 `reflect_export_session` 并短暂展示导出路径。
 */
import { useState } from 'react';
import type { ReflectSessionInfo } from '@/utils/commands';
import { useI18n } from '@/utils/i18n';
import s from './SessionItemMenu.module.css';

export interface SessionItemMenuProps {
  session: ReflectSessionInfo;
  onRename: (newName: string) => Promise<void>;
  onDelete: () => Promise<void>;
  onExport: () => Promise<string | null>;
  /** 可选：归档（移出会话列表，可在归档区恢复）。 */
  onArchive?: () => Promise<void>;
}

export function SessionItemMenu({ session, onRename, onDelete, onExport, onArchive }: SessionItemMenuProps) {
  const { t } = useI18n();
  const [renameOpen, setRenameOpen] = useState(false);
  // 预填充当前标题(自定义名或派生值;无标题的空会话为空串)。
  const [renameValue, setRenameValue] = useState(session.title ?? '');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);

  const run = async (fn: () => Promise<void>) => {
    setPending(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };

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

  const handleArchive = () =>
    run(async () => {
      if (confirm(t('threads.archiveConfirm', { id: session.session_id }))) {
        await onArchive?.();
      }
    });

  const handleDelete = () =>
    run(async () => {
      if (confirm(t('threads.deleteConfirm', { id: session.session_id }))) {
        await onDelete();
      }
    });

  const handleExport = () =>
    run(async () => {
      const path = await onExport();
      if (path) {
        setExportPath(path);
        setTimeout(() => setExportPath(null), 3000);
      }
    });

  if (renameOpen) {
    return (
      <div className={s.menu} data-session-menu={session.session_id} role="dialog" aria-label={t('threads.renameThread')}>
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
            data-testid={`session-rename-input-${session.session_id}`}
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
    <div className={s.menu} data-session-menu={session.session_id} role="menu" aria-label={t('threads.threadActions')}>
      <button
        type="button"
        role="menuitem"
        className={s.item}
        onClick={() => setRenameOpen(true)}
        disabled={pending}
        data-testid={`session-rename-${session.session_id}`}
      >
        ✎ {t('threads.rename')}
      </button>
      <button
        type="button"
        role="menuitem"
        className={s.item}
        onClick={() => void handleExport()}
        disabled={pending}
        data-testid={`session-export-${session.session_id}`}
      >
        ↓ {t('threads.export')}
      </button>
      {onArchive && (
        <button
          type="button"
          role="menuitem"
          className={s.item}
          onClick={() => void handleArchive()}
          disabled={pending}
          data-testid={`session-archive-${session.session_id}`}
        >
          📦 {t('threads.archive')}
        </button>
      )}
      <button
        type="button"
        role="menuitem"
        className={`${s.item} ${s.danger}`}
        onClick={() => void handleDelete()}
        disabled={pending}
        data-testid={`session-delete-${session.session_id}`}
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
