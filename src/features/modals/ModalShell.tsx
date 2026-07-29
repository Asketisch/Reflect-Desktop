/**
 * ModalShell —— 全屏遮罩 + 居中卡片（CSS Modules 版 + 焦点陷阱）。
 *
 * 阶段 A5 修复：
 *   - aria-modal="true" + role="dialog"。
 *   - 焦点陷阱：Tab/Shift+Tab 在 dialog 内循环；打开时聚焦 primary；关闭时还原焦点。
 *   - Enter 仅在 dialog 本身（非表单控件）聚焦时触发 primary，避免误提交。
 *   - Esc 关闭。
 *   - 点击遮罩关闭。
 */
import { useEffect, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { Icon, IconButton } from '@/features/design-system';
import s from './ModalShell.module.css';

interface Props {
  title: string;
  open: boolean;
  onClose: () => void;
  primaryAction?: { label: string; onClick: () => void; autoFocus?: boolean };
  secondaryAction?: { label: string; onClick: () => void };
  tertiaryAction?: { label: string; onClick: () => void };
  children: ReactNode;
}

const FOCUSABLE = 'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function ModalShell({
  title,
  open,
  onClose,
  primaryAction,
  secondaryAction,
  tertiaryAction,
  children,
}: Props) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const primaryRef = useRef<HTMLButtonElement>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    // 记录当前焦点，关闭时还原。
    restoreFocusRef.current = (document.activeElement as HTMLElement) ?? null;

    const dialog = dialogRef.current;
    // 初始聚焦 primary 按钮（或 dialog 本身）。
    const target = primaryRef.current ?? dialog;
    target?.focus();

    const getFocusable = (): HTMLElement[] => {
      if (!dialog) return [];
      return Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => el.offsetParent !== null || el === document.activeElement,
      );
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === 'Tab') {
        // 焦点陷阱：在 dialog 内循环。
        const focusable = getFocusable();
        if (focusable.length === 0) {
          e.preventDefault();
          dialog?.focus();
          return;
        }
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        const active = document.activeElement as HTMLElement;
        if (e.shiftKey) {
          if (active === first || !dialog?.contains(active)) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (active === last || !dialog?.contains(active)) {
            e.preventDefault();
            first.focus();
          }
        }
        return;
      }
      if (e.key === 'Enter' && primaryAction) {
        const active = document.activeElement as HTMLElement | null;
        const tag = active?.tagName;
        // 仅当焦点在 dialog 容器本身（dialog 自身或非表单控件子元素）时
        // 触发 primary。焦点跑出 dialog 时不接管,避免跨组件误提交。
        const insideDialog = !!active && !!dialog && dialog.contains(active);
        if (
          insideDialog &&
          (active === dialog ||
            (tag !== 'TEXTAREA' && tag !== 'INPUT' && tag !== 'SELECT' && tag !== 'BUTTON'))
        ) {
          e.preventDefault();
          primaryAction.onClick();
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      // 还原焦点。
      restoreFocusRef.current?.focus?.();
    };
  }, [open, onClose, primaryAction]);

  if (!open) return null;
  return (
    <div className={s.overlay} onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={s.dialog}
      >
        <header className={s.header}>
          <h2 className={s.title}>{title}</h2>
          <IconButton label="Close" size="sm" onClick={onClose}>
            <Icon icon={X} size={14} />
          </IconButton>
        </header>
        <div className={s.body}>{children}</div>
        {(tertiaryAction || secondaryAction || primaryAction) && (
          <footer className={s.footer}>
            {tertiaryAction && (
              <button className={s.btnGhost} onClick={tertiaryAction.onClick}>
                {tertiaryAction.label}
              </button>
            )}
            <div className={s.footerRight}>
              {secondaryAction && (
                <button className={s.btnSecondary} onClick={secondaryAction.onClick}>
                  {secondaryAction.label}
                </button>
              )}
              {primaryAction && (
                <button
                  ref={primaryRef}
                  className={s.btnPrimary}
                  onClick={primaryAction.onClick}
                  autoFocus={primaryAction.autoFocus}
                >
                  {primaryAction.label}
                </button>
              )}
            </div>
          </footer>
        )}
      </div>
    </div>
  );
}
