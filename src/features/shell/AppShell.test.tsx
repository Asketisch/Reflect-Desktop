/**
 * AppShell 折叠行为测试。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { AppShell } from '@/features/shell/AppShell';
import { useAgentStore } from '@/stores/agentStore';
import { resetMockInvoke, mockInvoke, createTestQueryClient } from '@/test/setup.tsx';

// Mock @tanstack/react-router — 仿照 app.smoke.test.tsx 的 mock 模式。
// Outlet 渲染一个 sentinel，验证 AppShell 真的把 router children 挂到了 DOM
// （历史 bug：用 children prop 而不是 <Outlet />，导致主区域永久空白）。
vi.mock('@tanstack/react-router', async () => {
  const actual = await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return {
    ...actual,
    Link: ({
      children,
      to,
      onClick,
      style,
      params: _params,
      ...rest
    }: {
      children: React.ReactNode;
      to: string;
      onClick?: () => void;
      style?: React.CSSProperties;
      params?: Record<string, unknown>;
      [k: string]: unknown;
    }) => (
      <a href={typeof to === 'string' ? to.replace(/\$/g, String(_params?.sessionId ?? '')) : '#'} onClick={onClick} style={style} {...rest}>
        {children}
      </a>
    ),
    useNavigate: () => () => {},
    useRouter: () => ({ navigate: vi.fn() }),
    useLocation: () => ({ pathname: '/' }),
    useMatches: () => [],
    Outlet: () => <div data-testid="outlet-content">outlet</div>,
  };
});

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('AppShell collapse behavior', () => {
  beforeEach(() => {
    resetMockInvoke();
    useAgentStore.getState().reset();
  });
  afterEach(() => cleanup());

  it('initial: sidebar visible, inspector hidden', () => {
    render(wrap(<AppShell />));
    expect(screen.getByTestId('shell-sidebar')).toBeDefined();
    expect(screen.queryByTestId('shell-inspector')).toBeNull();
  });

  it('clicking sidebar toggle button removes sidebar from DOM', () => {
    render(wrap(<AppShell />));
    expect(screen.getByTestId('shell-sidebar')).toBeDefined();
    fireEvent.click(screen.getByLabelText('Hide sidebar'));
    expect(screen.queryByTestId('shell-sidebar')).toBeNull();
  });

  it('clicking sidebar toggle twice round-trips (open → close → open)', () => {
    render(wrap(<AppShell />));
    const hideBtn = screen.getByLabelText('Hide sidebar');
    fireEvent.click(hideBtn);
    expect(screen.queryByTestId('shell-sidebar')).toBeNull();
    // 第一次折叠后，按钮 label 变回 "Show sidebar"
    fireEvent.click(screen.getByLabelText('Show sidebar'));
    expect(screen.getByTestId('shell-sidebar')).toBeDefined();
  });

  it('clicking inspector toggle adds inspector to DOM', () => {
    render(wrap(<AppShell />));
    expect(screen.queryByTestId('shell-inspector')).toBeNull();
    fireEvent.click(screen.getByLabelText('Show inspector'));
    expect(screen.getByTestId('shell-inspector')).toBeDefined();
  });

  it('both panels can be collapsed simultaneously', () => {
    render(wrap(<AppShell />));
    fireEvent.click(screen.getByLabelText('Hide sidebar'));
    fireEvent.click(screen.getByLabelText('Show inspector'));
    fireEvent.click(screen.getByLabelText('Hide inspector'));
    expect(screen.queryByTestId('shell-sidebar')).toBeNull();
    expect(screen.queryByTestId('shell-inspector')).toBeNull();
  });

  // 回归保护：AppShell 必须渲染 <Outlet />（不是 children prop），
  // 否则 TanStack Router 解析的子路由全部不显示 —— 主区域永久空白。
  it('renders Outlet content (router children actually mount)', () => {
    render(wrap(<AppShell />));
    expect(screen.getByTestId('outlet-content')).toBeDefined();
  });

  // v1.x workspace→session 归属：New Chat 主动分配 session id(纯 ID 分配,
  // session 在首条 submission 时后端物化), 再 setActiveId 导航到 /chat/$id。
  it('New Chat calls reflect_create_session before navigating', async () => {
    const created: unknown[] = [];
    mockInvoke('reflect_create_session', async () => {
      created.push('sess-new-1');
      return 'sess-new-1';
    });
    mockInvoke('reflect_list_sessions', async () => []);
    mockInvoke('reflect_current_workspace', async () => null);

    render(wrap(<AppShell />));
    fireEvent.click(screen.getByRole('button', { name: /new chat/i }));

    await new Promise((r) => setTimeout(r, 0)); // 让 .then(setActiveId) 微任务落地
    expect(created).toEqual(['sess-new-1']);
  });

  // 回归保护：TitleBar 必须挂 data-tauri-drag-region（否则窗口无法拖动），
  // 必须是 .shell 的第一个子元素（贯通全宽，红绿灯嵌在里面）。
  it('TitleBar is top-level (spans full width) and has drag region', () => {
    const { container } = render(wrap(<AppShell />));
    const titlebar = container.querySelector('[data-testid="titlebar"]');
    expect(titlebar).not.toBeNull();
    expect(titlebar?.hasAttribute('data-tauri-drag-region')).toBe(true);

    // .shell 的第一个子元素应当是 titlebar（不是 ActivityBar）。
    const shell = container.firstChild as HTMLElement;
    expect(shell.firstChild).toBe(titlebar);
  });
});