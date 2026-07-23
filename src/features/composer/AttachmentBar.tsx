/**
 * AttachmentBar —— Composer 底部附件预览条 (B5)。
 *
 * 显示当前待发的 attachments(local_image / inline image / skill mention)。
 * 每个 chip 有 "×" 按钮删除。
 */
import type { PendingAttachment } from './useAttachments';
import s from './AttachmentBar.module.css';

export interface AttachmentBarProps {
  attachments: PendingAttachment[];
  onRemove: (idx: number) => void;
}

export function AttachmentBar({ attachments, onRemove }: AttachmentBarProps) {
  if (attachments.length === 0) return null;

  return (
    <div className={s.bar} data-testid="attachment-bar">
      {attachments.map((a, idx) => {
        const label =
          a.kind === 'local_image'
            ? `🖼 ${shorten(a.path)}`
            : a.kind === 'image'
            ? `🖼 inline (${a.mime_type})`
            : `⚡ /${a.name}`;
        return (
          <div key={`${a.kind}-${idx}`} className={s.chip}>
            <span className={s.label}>{label}</span>
            <button
              type="button"
              className={s.remove}
              aria-label="Remove attachment"
              data-testid={`attachment-remove-${idx}`}
              onClick={() => onRemove(idx)}
            >
              ×
            </button>
          </div>
        );
      })}
    </div>
  );
}

function shorten(p: string): string {
  if (p.length <= 28) return p;
  return `…${p.slice(p.length - 26)}`;
}
