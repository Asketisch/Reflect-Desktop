/**
 * Vitest — ThreadsView 组件测试。
 *
 * 验证:
 * 1. 渲染 session buckets
 * 2. 空状态显示
 * 3. 显示 session 信息
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { ThreadsView } from '@/features/threads/ThreadsView';
import { resetMockInvoke, createTestQueryClient } from '@/test/setup.tsx';
import { useSessions } from '@/features/sessions/hooks/useSessions';

// Mock useSessions hook
vi.mock('@/features/sessions/hooks/useSessions', () => ({
  useSessions: vi.fn(),
}));

// Mock @tanstack/react-router Link
vi.mock('@tanstack/react-router', async () => {
  const actual = await vi.importActual('@tanstack/react-router');
  return {
    ...actual,
    Link: ({ children, to, onClick }: { children: React.ReactNode; to: string; onClick?: () => void }) => (
      <a href={to} onClick={onClick}>
        {children}
      </a>
    ),
  };
});

describe('ThreadsView', () => {
  beforeEach(() => {
    resetMockInvoke();
    vi.clearAllMocks();
  });

  it('renders session buckets with sessions', () => {
    const now = Date.now();
    vi.mocked(useSessions).mockReturnValue({
      buckets: [
        {
          label: 'Now',
          sessions: [
            {
              session_id: 's1',
              thread_id: 't1',
              model: 'stub/test',
              provider: 'local',
              started_at: new Date(now - 30 * 60 * 1000).toISOString(),
              message_count: 2,
              tool_count: 0,
              token_total: 0,
              cwd: '/tmp',
              display_name: 'Recent chat',
            },
          ],
        },
      ],
      all: [],
      loading: false,
      error: null,
      refetch: vi.fn(),
      refresh: vi.fn(),
      activeId: null,
      setActiveId: vi.fn(),
      rename: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <ThreadsView />
      </QueryClientProvider>
    );

    expect(screen.getByText('Threads')).toBeDefined();
    expect(screen.getByText('Recent chat')).toBeDefined();
    expect(screen.getByText(/2 msgs/)).toBeDefined();
    expect(screen.getByText('Now')).toBeDefined();
  });

  it('shows empty state when no sessions', () => {
    vi.mocked(useSessions).mockReturnValue({
      buckets: [],
      all: [],
      loading: false,
      error: null,
      refetch: vi.fn(),
      refresh: vi.fn(),
      activeId: null,
      setActiveId: vi.fn(),
      rename: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <ThreadsView />
      </QueryClientProvider>
    );

    expect(screen.getByText(/No threads yet/)).toBeDefined();
  });

  it('highlights active session', () => {
    const now = Date.now();
    vi.mocked(useSessions).mockReturnValue({
      buckets: [
        {
          label: 'Now',
          sessions: [
            {
              session_id: 's1',
              thread_id: 't1',
              model: 'stub/test',
              provider: 'local',
              started_at: new Date(now - 30 * 60 * 1000).toISOString(),
              message_count: 2,
              tool_count: 0,
              token_total: 0,
              cwd: '/tmp',
              display_name: 'Active chat',
            },
          ],
        },
      ],
      all: [],
      loading: false,
      error: null,
      refetch: vi.fn(),
      refresh: vi.fn(),
      activeId: 's1',
      setActiveId: vi.fn(),
      rename: vi.fn(),
    });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <ThreadsView />
      </QueryClientProvider>
    );

    expect(screen.getByText('Active chat')).toBeDefined();
    // Active session is rendered with bolder font weight (600 vs 400)
    const activeLink = screen.getByText('Active chat').closest('a');
    expect(activeLink).toBeDefined();
  });
});
