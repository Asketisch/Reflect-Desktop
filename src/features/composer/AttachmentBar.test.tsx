/**
 * AttachmentBar —— 附件预览条测试:inline image 缩略图 + 文字 chip + 删除。
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AttachmentBar } from './AttachmentBar';
import type { PendingAttachment } from './useAttachments';

const base: PendingAttachment[] = [
  { kind: 'image', data: 'data:image/png;base64,AAAA', mime_type: 'image/png' },
  { kind: 'file', path: 'src/app.tsx', range: { start_line: 1, end_line: 9 } },
  { kind: 'local_image', path: '(file) notes.txt' },
];

describe('AttachmentBar', () => {
  it('renders a real thumbnail for inline image attachments', () => {
    render(<AttachmentBar attachments={base} onRemove={vi.fn()} />);
    const thumb = screen.getByTestId('attachment-thumb') as HTMLImageElement;
    expect(thumb.src).toBe('data:image/png;base64,AAAA');
    expect(screen.getByTestId('attachment-bar').textContent).toContain('image/png');
  });

  it('renders text labels for file mentions and local images', () => {
    render(<AttachmentBar attachments={base} onRemove={vi.fn()} />);
    expect(screen.getByTestId('attachment-bar').textContent).toContain('src/app.tsx');
    expect(screen.getByTestId('attachment-bar').textContent).toContain('L1-9');
    expect(screen.getByTestId('attachment-bar').textContent).toContain('(file) notes.txt');
  });

  it('emits remove with the attachment index', () => {
    const onRemove = vi.fn();
    render(<AttachmentBar attachments={base} onRemove={onRemove} />);
    fireEvent.click(screen.getByTestId('attachment-remove-1'));
    expect(onRemove).toHaveBeenCalledWith(1);
  });

  it('renders nothing without attachments', () => {
    const { container } = render(<AttachmentBar attachments={[]} onRemove={vi.fn()} />);
    expect(container.querySelector('[data-testid="attachment-bar"]')).toBeNull();
  });
});
