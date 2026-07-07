/**
 * Vitest — MessageList 组件测试。
 *
 * 验证:
 * 1. 渲染空状态
 * 2. 显示 session 等待信息
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { MessageList } from '@/features/messages/MessageList';
import { createTestQueryClient } from '@/test/setup.tsx';
import { useAgent } from '@/services/agent';

// Mock useAgent hook
vi.mock('@/services/agent', () => ({
  useAgent: vi.fn(),
}));

describe('MessageList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders empty state initially', () => {
    vi.mocked(useAgent).mockReturnValue({
      turns: [],
      session: null,
      submit: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MessageList />
      </QueryClientProvider>
    );

    expect(screen.getByText('说点什么开始对话…')).toBeDefined();
  });

  it('shows waiting for session when no session', () => {
    vi.mocked(useAgent).mockReturnValue({
      turns: [],
      session: null,
      submit: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MessageList />
      </QueryClientProvider>
    );

    // Should show waiting session message
    const sessionTexts = screen.getAllByText(/session: \(waiting/);
    expect(sessionTexts.length).toBeGreaterThanOrEqual(1);
  });

  it('renders turns when agent has turns', () => {
    vi.mocked(useAgent).mockReturnValue({
      turns: [
        { id: 't1', user: 'hello', reply: 'Hi there!', done: true },
      ],
      session: { model: 'stub/test', provider: 'local' },
      submit: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MessageList />
      </QueryClientProvider>
    );

    expect(screen.getByText('hello')).toBeDefined();
    expect(screen.getByText('Hi there!')).toBeDefined();
  });
});
