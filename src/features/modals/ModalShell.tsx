/**
 * M1.6 ModalShell —— 全屏遮罩 + 居中卡片 + Esc 关闭 + Enter 主操作。
 *
 * M1.x 接受原生 dialog 形态(M2.x 再升级 @radix-ui/react-dialog 增加焦点陷阱)。
 */
import { useEffect, useRef, type ReactNode } from 'react';

interface Props {
  title: string;
  open: boolean;
  onClose: () => void;
  primaryAction?: { label: string; onClick: () => void; autoFocus?: boolean };
  secondaryAction?: { label: string; onClick: () => void };
  tertiaryAction?: { label: string; onClick: () => void };
  children: ReactNode;
}

export function ModalShell({
  title,
  open,
  onClose,
  primaryAction,
  secondaryAction,
  tertiaryAction,
  children,
}: Props) {
  const primaryRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    primaryRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
      else if (e.key === 'Enter' && primaryAction && document.activeElement?.tagName !== 'TEXTAREA') {
        primaryAction.onClick();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose, primaryAction]);

  if (!open) return null;
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 100,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-label={title}
        style={{
          background: 'white',
          padding: 20,
          borderRadius: 8,
          minWidth: 420,
          maxWidth: 560,
          maxHeight: '80vh',
          overflow: 'auto',
          boxShadow: '0 12px 48px rgba(0,0,0,0.3)',
        }}
      >
        <h2 style={{ marginTop: 0 }}>{title}</h2>
        <div>{children}</div>
        <div
          style={{
            marginTop: 16,
            display: 'flex',
            justifyContent: 'flex-end',
            gap: 8,
          }}
        >
          {tertiaryAction && (
            <button onClick={tertiaryAction.onClick}>{tertiaryAction.label}</button>
          )}
          {secondaryAction && (
            <button onClick={secondaryAction.onClick}>{secondaryAction.label}</button>
          )}
          {primaryAction && (
            <button
              ref={primaryRef}
              onClick={primaryAction.onClick}
              autoFocus={primaryAction.autoFocus}
              style={{
                background: '#3b82f6',
                color: 'white',
                border: 'none',
                padding: '6px 14px',
                borderRadius: 4,
              }}
            >
              {primaryAction.label}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
