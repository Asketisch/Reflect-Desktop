/**
 * Toast —— 单条 toast。
 *
 * CodexMonitor 同名: `src/components/Toast.tsx`
 */
import { useEffect } from 'react';
import { TOAST_COLORS, type ToastKind, DEFAULT_TOAST_MS } from '../utils/toast';

export interface ToastProps {
  kind: ToastKind;
  message: string;
  durationMs?: number;
  onDismiss: () => void;
}

export function Toast({ kind, message, durationMs = DEFAULT_TOAST_MS, onDismiss }: ToastProps) {
  useEffect(() => {
    const t = setTimeout(onDismiss, durationMs);
    return () => clearTimeout(t);
  }, [durationMs, onDismiss]);

  const c = TOAST_COLORS[kind];
  return (
    <div
      role="status"
      style={{
        padding: '8px 12px',
        background: c.bg,
        color: c.fg,
        border: `1px solid ${c.border}`,
        borderRadius: 6,
        fontSize: 13,
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        minWidth: 200,
        maxWidth: 360,
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.08)',
      }}
      data-kind={kind}
    >
      <span style={{ flex: 1 }}>{message}</span>
      <button
        onClick={onDismiss}
        aria-label="Dismiss"
        style={{
          background: 'transparent',
          border: 0,
          color: c.fg,
          cursor: 'pointer',
          fontSize: 14,
          padding: 0,
        }}
      >
        ×
      </button>
    </div>
  );
}