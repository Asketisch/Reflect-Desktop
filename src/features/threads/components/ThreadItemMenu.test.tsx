/**
 * Vitest — ThreadItemMenu 测试。
 *
 * 验证:
 * 1. 三个动作菜单项 (rename / export / delete) 渲染
 * 2. rename → 提交 input → onRename 调用 + new value
 * 3. export → onExport 调用 + toast 显示
 * 4. delete → confirm 拒绝时不会调用 onDelete
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ThreadItemMenu } from '@/features/threads/components/ThreadItemMenu';
import type { ReflectSessionInfo } from '@/utils/tauri';

const SESSION: ReflectSessionInfo = {
  session_id: 's1',
  thread_id: 't1',
  model: 'stub/test',
  provider: 'local',
  started_at: new Date().toISOString(),
  message_count: 2,
  tool_count: 0,
  token_total: 0,
  cwd: '/tmp',
  display_name: 'Recent chat',
};

describe('ThreadItemMenu', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders three actions', () => {
    render(
      <ThreadItemMenu
        session={SESSION}
        onRename={vi.fn().mockResolvedValue(undefined)}
        onDelete={vi.fn().mockResolvedValue(undefined)}
        onExport={vi.fn().mockResolvedValue('/tmp/out.json')}
      />,
    );
    expect(screen.getByRole('menuitem', { name: /Rename/ })).toBeDefined();
    expect(screen.getByRole('menuitem', { name: /Export/ })).toBeDefined();
    expect(screen.getByRole('menuitem', { name: /Delete/ })).toBeDefined();
  });

  it('opens rename input and submits new name', async () => {
    const onRename = vi.fn().mockResolvedValue(undefined);
    const { container } = render(
      <ThreadItemMenu
        session={SESSION}
        onRename={onRename}
        onDelete={vi.fn().mockResolvedValue(undefined)}
        onExport={vi.fn().mockResolvedValue('/tmp/out.json')}
      />,
    );

    const renameBtn = container.querySelector('[data-testid="thread-rename-s1"]') as HTMLButtonElement;
    fireEvent.click(renameBtn);
    const input = (await waitFor(() =>
      container.querySelector('[data-testid="thread-rename-input-s1"]'),
    )) as HTMLInputElement;

    fireEvent.change(input, { target: { value: 'Renamed chat' } });
    const form = input.closest('form')!;
    fireEvent.submit(form);

    await waitFor(() => expect(onRename).toHaveBeenCalledWith('Renamed chat'));
  });

  it('shows toast on export', async () => {
    const { container } = render(
      <ThreadItemMenu
        session={SESSION}
        onRename={vi.fn().mockResolvedValue(undefined)}
        onDelete={vi.fn().mockResolvedValue(undefined)}
        onExport={vi.fn().mockResolvedValue('/tmp/s1.json')}
      />,
    );

    const exportBtn = container.querySelector('[data-testid="thread-export-s1"]') as HTMLButtonElement;
    fireEvent.click(exportBtn);

    await waitFor(() => {
      const text = container.textContent ?? '';
      expect(text).toContain('Exported');
      expect(text).toContain('/tmp/s1.json');
    });
  });

  it('does not delete when confirm() returns false', async () => {
    const onDelete = vi.fn().mockResolvedValue(undefined);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const { container } = render(
      <ThreadItemMenu
        session={SESSION}
        onRename={vi.fn().mockResolvedValue(undefined)}
        onDelete={onDelete}
        onExport={vi.fn().mockResolvedValue(null)}
      />,
    );

    const deleteBtn = container.querySelector('[data-testid="thread-delete-s1"]') as HTMLButtonElement;
    fireEvent.click(deleteBtn);
    expect(onDelete).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('deletes when confirm() returns true', async () => {
    const onDelete = vi.fn().mockResolvedValue(undefined);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const { container } = render(
      <ThreadItemMenu
        session={SESSION}
        onRename={vi.fn().mockResolvedValue(undefined)}
        onDelete={onDelete}
        onExport={vi.fn().mockResolvedValue(null)}
      />,
    );

    const deleteBtn = container.querySelector('[data-testid="thread-delete-s1"]') as HTMLButtonElement;
    fireEvent.click(deleteBtn);

    await waitFor(() => expect(onDelete).toHaveBeenCalled());
    confirmSpy.mockRestore();
  });
});
