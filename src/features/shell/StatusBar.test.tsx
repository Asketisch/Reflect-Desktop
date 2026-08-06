/**
 * StatusBar —— token/cost 显示测试。
 *
 * 验证：
 *   - tokens === null 时不渲染 token 指示器
 *   - tokens 有值时显示 total + cost
 *   - cost === null 时只显示 total,不含 "$"
 *
 * 仿 AppShell.test.tsx 模式：QueryClient 包裹 + resetMockInvoke + store.setState。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { StatusBar } from '@/features/shell/StatusBar';
import { useAgentStore } from '@/stores/agentStore';
import { resetMockInvoke, createTestQueryClient } from '@/test/setup.tsx';
import type { TokenSnapshot } from '@/stores/agent/types';

vi.mock('@tanstack/react-router', async () => {
  const actual = await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return { ...actual, Outlet: () => null, Link: () => null, useNavigate: () => () => {} };
});

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('StatusBar — token indicator', () => {
  beforeEach(() => {
    resetMockInvoke();
    useAgentStore.getState().reset();
  });
  afterEach(() => cleanup());

  it('does not render token indicator when tokens is null', () => {
    useAgentStore.setState({ tokens: null });
    render(wrap(<StatusBar />));
    expect(screen.queryByTestId('statusbar-tokens')).toBeNull();
  });

  it('renders total + cost when tokens present', () => {
    const tokens: TokenSnapshot = {
      input: 1200,
      output: 300,
      cached: 200,
      cacheWrite: 500,
      total: 1500,
      cost: 0.0123,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    const el = screen.getByTestId('statusbar-tokens');
    const text = el.textContent ?? '';
    expect(text).toContain('1,500'); // total
    expect(text).toContain('$0.0123'); // cost
  });

  it('renders total only when cost is null', () => {
    const tokens: TokenSnapshot = {
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: null,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    const el = screen.getByTestId('statusbar-tokens');
    const text = el.textContent ?? '';
    expect(text).toContain('15');
    expect(text).not.toContain('$');
  });
});
