/**
 * Vitest — 会话置顶（pins）纯函数 + 侧边栏置顶区。
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { I18nProvider } from '@/utils/i18n';
import { isPinned, readPins, resetPinsForTests, togglePin, unpin, useSessionPins } from '@/features/sessions/utils/pins';
import type { WorkspaceSessionGroup } from '@/features/sessions/utils/workspaceGroups';
import { Sidebar } from '@/features/sessions/components/Sidebar';

beforeEach(() => {
  window.localStorage.clear();
  resetPinsForTests();
});

describe('pins utils', () => {
  it('toggles pin state and preserves order', () => {
    expect(readPins()).toEqual([]);
    expect(togglePin('a')).toBe(true);
    expect(togglePin('b')).toBe(true);
    // 取消置顶移除，重新置顶追加在尾部（置顶顺序 = 操作顺序）。
    expect(togglePin('a')).toBe(false);
    expect(readPins()).toEqual(['b']);
    expect(togglePin('a')).toBe(true);
    expect(readPins()).toEqual(['b', 'a']);
    expect(isPinned('a')).toBe(true);
    expect(togglePin('b')).toBe(false);
    expect(readPins()).toEqual(['a']);
  });

  it('unpin removes dangling references', () => {
    togglePin('a');
    unpin('a');
    unpin('missing'); // 幂等
    expect(readPins()).toEqual([]);
  });
});

function sess(id: string, title?: string) {
  return {
    session_id: id,
    model: 'stub/test',
    started_at: new Date().toISOString(),
    message_count: 2,
    ...(title ? { title } : {}),
  };
}

const groupA: WorkspaceSessionGroup = {
  path: '/Users/me/Code/alpha',
  key: '/Users/me/Code/alpha',
  label: 'alpha',
  sessions: [sess('s1', 'Fix login bug'), sess('s2', 'Explore codebase')],
  lastActivity: Date.now(),
};

function renderSidebar(props: Partial<Parameters<typeof Sidebar>[0]> = {}) {
  return render(
    <I18nProvider>
      <Sidebar
        groups={[groupA]}
        loading={false}
        error={null}
        activeId={null}
        currentWorkspace="/Users/me/Code/alpha"
        onSelect={() => {}}
        onRefresh={() => {}}
        {...props}
      />
    </I18nProvider>,
  );
}

describe('Sidebar pinned section', () => {
  it('renders pinned sessions on top and removes them from project groups', () => {
    const { container } = renderSidebar({ pinnedIds: ['s1'], onTogglePin: () => {} });
    const pinnedSection = screen.getByText('Pinned');
    expect(pinnedSection).toBeDefined();
    // 置顶会话出现在置顶区（带 📍 标记）。
    const pinnedGroup = container.querySelector('section')!;
    expect(pinnedGroup.textContent).toContain('Fix login bug');
    // 常规分组不再包含 s1。
    const alphaGroup = Array.from(container.querySelectorAll('section')).find((el) =>
      el.textContent?.includes('alpha'),
    )!;
    expect(alphaGroup.textContent).not.toContain('📍 Fix login bug');
  });

  it('hides pinned section when nothing is pinned', () => {
    renderSidebar({});
    expect(screen.queryByText('Pinned')).toBeNull();
  });

  it('archive section lists archived sessions with restore label', () => {
    renderSidebar({
      archived: [sess('arch-1', 'Old work')],
      onUnarchive: async () => {},
      onDelete: async () => {},
      onExport: async () => null,
      onRename: async () => {},
    });
    fireEvent.click(screen.getByTestId('sidebar-archived').querySelector('button')!);
    expect(screen.getByText('Old work')).toBeDefined();
    // 归档行的菜单项语义为「恢复」。
    fireEvent.click(screen.getByTestId('session-kebab-arch-1'));
    expect(screen.getByTestId('session-archive-arch-1').textContent).toContain('Restore from archive');
  });
});

describe('useSessionPins hook', () => {
  it('reflects toggles via the subscription', async () => {
    const probe: string[][] = [];
    function Probe() {
      const { pinnedIds, toggle } = useSessionPins();
      probe.push(pinnedIds);
      return <button onClick={() => toggle('x')} data-testid="probe-toggle" />;
    }
    const { getByTestId } = render(<Probe />);
    fireEvent.click(getByTestId('probe-toggle'));
    expect(probe[probe.length - 1]).toEqual(['x']);
  });
});
