/**
 * Vitest — Sidebar 组件测试（项目目录分组版）。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { ConfirmDialogHost } from '@/features/modals/ConfirmDialog';
import type { WorkspaceSessionGroup } from '@/features/sessions/utils/workspaceGroups';
import { UNASSIGNED_GROUP_KEY } from '@/features/sessions/utils/workspaceGroups';

const WS_A = '/Users/me/Code/alpha';
const WS_B = '/Users/me/Code/beta';

function sess(id: string, workspace: string | null, title?: string) {
  return {
    session_id: id,
    model: 'stub/test',
    started_at: new Date().toISOString(),
    message_count: 2,
    ...(title ? { title } : {}),
    ...(workspace === null ? {} : { workspace }),
  };
}

const groupA: WorkspaceSessionGroup = {
  path: WS_A,
  key: WS_A,
  label: 'alpha',
  sessions: [sess('s1', WS_A, 'Fix login bug')],
  lastActivity: Date.now(),
};

const groupB: WorkspaceSessionGroup = {
  path: WS_B,
  key: WS_B,
  label: 'beta',
  sessions: [sess('s2', WS_B)],
  lastActivity: 1,
};

const unassigned: WorkspaceSessionGroup = {
  path: null,
  key: UNASSIGNED_GROUP_KEY,
  label: '',
  sessions: [sess('u1', null)],
  lastActivity: 1,
};

beforeEach(() => {
  localStorage.clear();
});

describe('Sidebar', () => {
  it('renders empty state when no groups', () => {
    render(
      <Sidebar
        groups={[]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText(/No sessions yet/)).toBeDefined();
  });

  it('renders loading state', () => {
    render(
      <Sidebar
        groups={[]}
        loading={true}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText(/Loading/)).toBeDefined();
  });

  it('renders error message', () => {
    render(
      <Sidebar
        groups={[]}
        loading={false}
        error="boom"
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('boom')).toBeDefined();
  });

  it('renders project group header and session item', () => {
    render(
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId="s1"
        currentWorkspace={WS_A}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('alpha')).toBeDefined();
    expect(screen.getByText('Fix login bug')).toBeDefined();
    expect(screen.getByText(/2 msgs/)).toBeDefined();
    // 当前工作区分组带标记。
    expect(screen.getByLabelText('Current workspace')).toBeDefined();
    // 组头 hover 全路径。
    expect(screen.getByText('alpha').closest('button')?.getAttribute('title')).toBe(WS_A);
  });

  it('unassigned group is collapsed by default and expands on toggle', () => {
    render(
      <Sidebar
        groups={[groupA, unassigned]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('Unassigned')).toBeDefined();
    // 默认折叠:未归属会话不可见。
    expect(screen.queryByText('u1')).toBeNull();
    // 点组头展开。
    fireEvent.click(screen.getByText('Unassigned').closest('button')!);
    expect(screen.getByText('u1')).toBeDefined();
  });

  it('toggles an expanded project group collapsed and back', () => {
    render(
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('Fix login bug')).toBeDefined();
    fireEvent.click(screen.getByText('alpha').closest('button')!);
    expect(screen.queryByText('Fix login bug')).toBeNull();
    fireEvent.click(screen.getByText('alpha').closest('button')!);
    expect(screen.getByText('Fix login bug')).toBeDefined();
  });

  it('calls onNewChatIn with the project path from the group "+" button', () => {
    const onNewChatIn = vi.fn();
    render(
      <Sidebar
        groups={[groupA, unassigned]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
        onNewChatIn={onNewChatIn}
      />,
    );
    const plusBtns = screen.getAllByLabelText('New chat in this project');
    // 只有真实项目组有「+」,未归属分组没有。
    expect(plusBtns).toHaveLength(1);
    fireEvent.click(plusBtns[0]);
    expect(onNewChatIn).toHaveBeenCalledWith(WS_A);
  });

  it('expands all matched groups while searching by title', () => {
    render(
      <Sidebar
        groups={[groupA, groupB]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    const input = screen.getByLabelText('Search sessions');
    fireEvent.change(input, { target: { value: 'Fix login' } });
    // 只剩 alpha 组。
    expect(screen.getByText('alpha')).toBeDefined();
    expect(screen.queryByText('beta')).toBeNull();
    expect(screen.getByText('Fix login bug')).toBeDefined();
  });

  it('search matches project name so empty groups are discoverable', () => {
    const emptyBeta: WorkspaceSessionGroup = { ...groupB, sessions: [] };
    render(
      <Sidebar
        groups={[groupA, emptyBeta]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText('Search sessions'), { target: { value: 'beta' } });
    expect(screen.getByText('beta')).toBeDefined();
    expect(screen.queryByText('alpha')).toBeNull();
  });

  it('calls onSelect when item clicked', () => {
    const onSelect = vi.fn();
    const { container } = render(
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={onSelect}
        onRefresh={vi.fn()}
      />,
    );
    const link = container.querySelector('button[aria-pressed]') as HTMLButtonElement;
    link.click();
    expect(onSelect).toHaveBeenCalledWith('s1');
  });

  it('calls onRefresh when refresh button clicked', () => {
    const onRefresh = vi.fn();
    const { container } = render(
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={onRefresh}
      />,
    );
    const refreshBtn = container.querySelector('button[title="Refresh"]') as HTMLButtonElement;
    refreshBtn.click();
    expect(onRefresh).toHaveBeenCalled();
  });

  it('renders kebab menu with archive action when handlers provided', async () => {
    const onArchive = vi.fn().mockResolvedValue(undefined);
    const { container } = render(
      <>
        <Sidebar
          groups={[groupA]}
          loading={false}
          error={null}
          activeId={null}
          currentWorkspace={null}
          onSelect={vi.fn()}
          onRefresh={vi.fn()}
          onRename={vi.fn().mockResolvedValue(undefined)}
          onDelete={vi.fn().mockResolvedValue(undefined)}
          onExport={vi.fn().mockResolvedValue(null)}
          onArchive={onArchive}
        />
        <ConfirmDialogHost />
      </>,
    );
    const kebab = container.querySelector('[data-testid="session-kebab-s1"]') as HTMLButtonElement;
    expect(kebab).toBeDefined();
    kebab.click();
    const archiveBtn = await screen.findByTestId('session-archive-s1');
    fireEvent.click(archiveBtn);
    // 应用内确认框弹出 → 确认。
    fireEvent.click(await screen.findByTestId('confirm-dialog-confirm'));
    await waitFor(() => expect(onArchive).toHaveBeenCalledWith('s1'));
  });

  it('renders the open-project footer button and invokes the handler', () => {
    const onOpenProject = vi.fn();
    render(
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
        onOpenProject={onOpenProject}
      />,
    );

    const btn = screen.getByTestId('sidebar-open-project');
    fireEvent.click(btn);
    expect(onOpenProject).toHaveBeenCalledTimes(1);
  });

  it('multi-select mode bulk-deletes selected sessions', async () => {
    const onDelete = vi.fn().mockResolvedValue(undefined);
    const onSelect = vi.fn();
    render(
      <>
        <Sidebar
          groups={[groupA, groupB]}
          loading={false}
          error={null}
          activeId={null}
          currentWorkspace={null}
          onSelect={onSelect}
          onRefresh={vi.fn()}
          onRename={vi.fn().mockResolvedValue(undefined)}
          onDelete={onDelete}
          onExport={vi.fn().mockResolvedValue(null)}
        />
        <ConfirmDialogHost />
      </>,
    );

    // 进入多选 → 勾选两个会话。
    fireEvent.click(screen.getByTestId('sidebar-select-mode'));
    fireEvent.click(screen.getByTestId('session-select-s1'));
    fireEvent.click(screen.getByTestId('session-select-s2'));
    expect(screen.getByTestId('sidebar-selected-count').textContent).toContain('2');

    // 选择模式下点行 = 切换选中（再次点击取消勾选），不触发导航。
    fireEvent.click(screen.getByText('Fix login bug'));
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByTestId('sidebar-selected-count').textContent).toContain('1');
    fireEvent.click(screen.getByText('Fix login bug'));
    expect(screen.getByTestId('sidebar-selected-count').textContent).toContain('2');

    // 批量删除：应用内确认 → 逐条 onDelete，完成后退出多选。
    fireEvent.click(screen.getByTestId('sidebar-delete-selected'));
    fireEvent.click(await screen.findByTestId('confirm-dialog-confirm'));
    await waitFor(() => expect(onDelete).toHaveBeenCalledTimes(2));
    expect(onDelete).toHaveBeenCalledWith('s1');
    expect(onDelete).toHaveBeenCalledWith('s2');
    expect(screen.queryByTestId('sidebar-select-bar')).toBeNull();
  });
});
