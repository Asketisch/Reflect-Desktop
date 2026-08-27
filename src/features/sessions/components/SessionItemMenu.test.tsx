/**
 * Vitest — SessionItemMenu 测试（Sidebar 与 ThreadsView 共用的操作菜单）。
 *
 * 验证:
 * 1. 菜单项渲染（rename / export / delete；onArchive 存在时含 archive）
 * 2. rename → 提交 input → onRename 调用 + new value
 * 3. export → onExport 调用 + toast 显示
 * 4. delete / archive → confirm 拒绝时不调用
 * 5. archive → confirm 同意后调用 onArchive
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SessionItemMenu } from '@/features/sessions/components/SessionItemMenu';
import type { ReflectSessionInfo } from '@/utils/commands';

const SESSION: ReflectSessionInfo = {
  session_id: 's1',
  model: 'stub/test',
  started_at: new Date().toISOString(),
  message_count: 2,
};

const baseProps = () => ({
  session: SESSION,
  onRename: vi.fn().mockResolvedValue(undefined),
  onDelete: vi.fn().mockResolvedValue(undefined),
  onExport: vi.fn().mockResolvedValue('/tmp/out.json'),
});

describe('SessionItemMenu', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders core actions; archive only when handler provided', () => {
    const props = baseProps();
    const { rerender } = render(<SessionItemMenu {...props} />);
    expect(screen.getByRole('menuitem', { name: /Rename/ })).toBeDefined();
    expect(screen.getByRole('menuitem', { name: /Export/ })).toBeDefined();
    expect(screen.getByRole('menuitem', { name: /Delete/ })).toBeDefined();
    expect(screen.queryByRole('menuitem', { name: /Archive/ })).toBeNull();

    rerender(<SessionItemMenu {...props} onArchive={vi.fn().mockResolvedValue(undefined)} />);
    expect(screen.getByRole('menuitem', { name: /Archive/ })).toBeDefined();
  });

  it('opens rename input and submits new name', async () => {
    const onRename = vi.fn().mockResolvedValue(undefined);
    const { container } = render(
      <SessionItemMenu {...baseProps()} onRename={onRename} />,
    );

    const renameBtn = container.querySelector('[data-testid="session-rename-s1"]') as HTMLButtonElement;
    fireEvent.click(renameBtn);
    const input = (await waitFor(() =>
      container.querySelector('[data-testid="session-rename-input-s1"]'),
    )) as HTMLInputElement;

    fireEvent.change(input, { target: { value: 'Renamed chat' } });
    const form = input.closest('form')!;
    fireEvent.submit(form);

    await waitFor(() => expect(onRename).toHaveBeenCalledWith('Renamed chat'));
  });

  it('shows toast on export', async () => {
    const { container } = render(<SessionItemMenu {...baseProps()} />);

    const exportBtn = container.querySelector('[data-testid="session-export-s1"]') as HTMLButtonElement;
    fireEvent.click(exportBtn);

    await waitFor(() => {
      const text = container.textContent ?? '';
      expect(text).toContain('Exported');
      expect(text).toContain('/tmp/out.json');
    });
  });

  it('does not delete when confirm() returns false', async () => {
    const onDelete = vi.fn().mockResolvedValue(undefined);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const { container } = render(<SessionItemMenu {...baseProps()} onDelete={onDelete} />);

    const deleteBtn = container.querySelector('[data-testid="session-delete-s1"]') as HTMLButtonElement;
    fireEvent.click(deleteBtn);
    expect(onDelete).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('archives after confirm() accepts', async () => {
    const onArchive = vi.fn().mockResolvedValue(undefined);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const { container } = render(<SessionItemMenu {...baseProps()} onArchive={onArchive} />);

    const archiveBtn = container.querySelector('[data-testid="session-archive-s1"]') as HTMLButtonElement;
    fireEvent.click(archiveBtn);

    await waitFor(() => expect(onArchive).toHaveBeenCalled());
    confirmSpy.mockRestore();
  });

  it('does not archive when confirm() returns false', async () => {
    const onArchive = vi.fn().mockResolvedValue(undefined);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const { container } = render(<SessionItemMenu {...baseProps()} onArchive={onArchive} />);

    const archiveBtn = container.querySelector('[data-testid="session-archive-s1"]') as HTMLButtonElement;
    fireEvent.click(archiveBtn);
    expect(onArchive).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});
