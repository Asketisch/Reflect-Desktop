/**
 * AppShell 折叠行为测试。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { AppShell } from '@/features/shell/AppShell';
import { useAgentStore } from '@/stores/agentStore';
import { resetMockInvoke, createTestQueryClient } from '@/test/setup.tsx';

// Mock @tanstack/react-router — 仿照 app.smoke.test.tsx 的 mock 模式。
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
    Outlet: () => null,
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
});