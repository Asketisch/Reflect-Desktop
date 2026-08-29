/**
 * ConfirmDialog —— 应用内确认对话框（promise 式，替代 window.confirm）。
 *
 * 为什么不用 window.confirm：Tauri 桌面端的 WKWebView（wry）不实现
 * `WKUIDelegate` 的 JS 对话框回调，`window.confirm()` 在 macOS（以及
 * 未弹原生框的环境）不显示任何对话框且恒返回 false —— 此前所有依赖
 * 它的删除/归档确认都会静默失败（点击后无任何反应）。
 *
 * 用法：
 *   const ok = await confirmDialog({
 *     title: t('threads.delete'),
 *     message: t('threads.deleteSelectedConfirm', { n: '3' }),
 *     confirmLabel: t('common.delete'),
 *   });
 *   if (!ok) return;
 *
 * 需要在应用根部挂载一次 <ConfirmDialogHost />（AppShell，与 ModalStack 并列）。
 * 确认 → resolve(true)；取消 / Esc / 点遮罩 → resolve(false)。
 * 并发调用时前一个请求以 false 结束（仅保留最后一个）。
 */
import { useSyncExternalStore } from 'react';
import { useI18n } from '@/utils/i18n';
import { ModalShell } from './ModalShell';

export interface ConfirmOptions {
  title: string;
  message: string;
  /** 确认按钮文案（默认 common.confirm）。 */
  confirmLabel?: string;
  /** 取消按钮文案（默认 common.cancel）。 */
  cancelLabel?: string;
}

interface PendingRequest extends ConfirmOptions {
  resolve: (ok: boolean) => void;
}

let pending: PendingRequest | null = null;
const listeners = new Set<() => void>();

function emitChange() {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function settle(ok: boolean) {
  pending?.resolve(ok);
  pending = null;
  emitChange();
}

/** 弹出确认框并等待用户选择。 */
export function confirmDialog(opts: ConfirmOptions): Promise<boolean> {
  if (pending) pending.resolve(false);
  return new Promise<boolean>((resolve) => {
    pending = { ...opts, resolve };
    emitChange();
  });
}

/** 应用根部的宿主组件：有确认请求时渲染 ModalShell。 */
export function ConfirmDialogHost() {
  const request = useSyncExternalStore(
    subscribe,
    () => pending,
    () => pending,
  );
  const { t } = useI18n();
  if (!request) return null;
  return (
    <ModalShell
      title={request.title}
      open={true}
      onClose={() => settle(false)}
      secondaryAction={{
        label: request.cancelLabel ?? t('common.cancel'),
        onClick: () => settle(false),
        dataTestId: 'confirm-dialog-cancel',
      }}
      primaryAction={{
        label: request.confirmLabel ?? t('common.confirm'),
        onClick: () => settle(true),
        autoFocus: true,
        dataTestId: 'confirm-dialog-confirm',
      }}
    >
      <p>{request.message}</p>
    </ModalShell>
  );
}
