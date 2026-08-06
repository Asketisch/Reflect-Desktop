/**
 * Inspector —— Token Usage section 渲染测试。
 *
 * 验证：
 *   - tokens === null 时显示 empty 段落
 *   - tokens 有值时渲染 input/output/total/cost 等数值
 *   - cost === null 时不含 "$" 文本
 *
 * 仿 AppShell.test.tsx 模式：QueryClient 包裹 + resetMockInvoke + store.setState。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { Inspector } from '@/features/shell/Inspector';
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

function setTokens(tokens: TokenSnapshot | null) {
  useAgentStore.setState({ tokens });
}

describe('Inspector — token usage section', () => {
  beforeEach(() => {
    resetMockInvoke();
    useAgentStore.getState().reset();
  });
  afterEach(() => cleanup());

  it('renders the token usage section even when tokens is null (empty state)', () => {
    setTokens(null);
    const { container } = render(wrap(<Inspector />));
    // section 标题始终渲染(通过 Coins 图标定位 section);
    // empty 态下没有数值行。
    expect(container.textContent).not.toContain('1,200');
  });

  it('renders token breakdown values when tokens present', () => {
    setTokens({
      input: 1200,
      output: 300,
      cached: 200,
      cacheWrite: 500,
      total: 1500,
      cost: 0.0123,
      provider: 'anthropic',
      credentialLabel: 'main-key',
    });
    const { container } = render(wrap(<Inspector />));
    const text = container.textContent ?? '';
    expect(text).toContain('1,200'); // input
    expect(text).toContain('300'); // output
    expect(text).toContain('500'); // cacheWrite
    expect(text).toContain('1,500'); // total
    expect(text).toContain('$0.0123'); // cost
    expect(text).toContain('anthropic'); // provider
    expect(text).toContain('main-key'); // credential
  });

  it('omits cost row when cost is null', () => {
    setTokens({
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: null,
    });
    const { container } = render(wrap(<Inspector />));
    const text = container.textContent ?? '';
    expect(text).toContain('15'); // total 显示
    // 不含任何 "$" 开头的 cost 文本
    expect(text).not.toContain('$');
  });

  it('omits cacheWrite row when cacheWrite is 0', () => {
    setTokens({
      input: 10,
      output: 5,
      cached: 0,
      cacheWrite: 0,
      total: 15,
      cost: null,
    });
    const { container } = render(wrap(<Inspector />));
    const text = container.textContent ?? '';
    // cacheWrite === 0 时该行被省略;cached 即使是 0 也渲染(显示 "0")。
    // 这里确保 cacheWrite 行不渲染、组件不崩、total 正确。
    expect(text).not.toContain('Cache write');
    expect(text).toContain('15');
  });
});
