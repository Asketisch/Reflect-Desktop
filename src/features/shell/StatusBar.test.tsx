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

  it('renders total without any cost fragment when tokens present', () => {
    const tokens: TokenSnapshot = {
      input: 1200,
      output: 300,
      cached: 200,
      cacheWrite: 500,
      total: 1500,
      cost: null,
      sessionCost: 0,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    const el = screen.getByTestId('statusbar-tokens');
    const text = el.textContent ?? '';
    expect(text).toContain('1,500'); // total
    expect(text).not.toContain('$');
  });

  it('renders total only when cost is null', () => {
    const tokens: TokenSnapshot = {
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: null,
      sessionCost: 0,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    const el = screen.getByTestId('statusbar-tokens');
    const text = el.textContent ?? '';
    expect(text).toContain('15');
    expect(text).not.toContain('$');
  });

  it('renders cumulative session cost element when sessionCost > 0', () => {
    const tokens: TokenSnapshot = {
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: 0.0021,
      sessionCost: 0.0357,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    // 累计费用徽标出现且显示美元金额(4 位小数保留小额精度)。
    const cost = screen.getByTestId('statusbar-cost');
    expect(cost.textContent).toContain('$0.0357');
    // tooltip 含最近一次费用与累计费用两行。
    const tokensEl = screen.getByTestId('statusbar-tokens');
    expect(tokensEl).toBeTruthy();
  });

  it('hides cost element for unpriced models (cost null, sessionCost 0)', () => {
    const tokens: TokenSnapshot = {
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: null,
      sessionCost: 0,
    };
    useAgentStore.setState({ tokens });
    render(wrap(<StatusBar />));
    expect(screen.queryByTestId('statusbar-cost')).toBeNull();
  });
});
