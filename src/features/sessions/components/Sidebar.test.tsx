/**
 * Vitest — Sidebar 组件测试。
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import type { SessionBucket } from '@/features/sessions/utils/buckets';

const sample: SessionBucket[] = [
  {
    label: 'Now',
    sessions: [
      {
        session_id: 's1',
        model: 'stub/test',
        started_at: new Date().toISOString(),
        message_count: 2,
      },
    ],
  },
];

describe('Sidebar', () => {
  it('renders empty state when no buckets', () => {
    render(
      <Sidebar
        buckets={[]}
        loading={false}
        error={null}
        activeId={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText(/No sessions yet/)).toBeDefined();
  });

  it('renders loading state', () => {
    render(
      <Sidebar
        buckets={[]}
        loading={true}
        error={null}
        activeId={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText(/Loading/)).toBeDefined();
  });

  it('renders error message', () => {
    render(
      <Sidebar
        buckets={[]}
        loading={false}
        error="boom"
        activeId={null}
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('boom')).toBeDefined();
  });

  it('renders bucket and item', () => {
    render(
      <Sidebar
        buckets={sample}
        loading={false}
        error={null}
        activeId="s1"
        onSelect={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(screen.getByText('Now')).toBeDefined();
    // session_id "s1" → displayTitle 返回 id 前缀。
    expect(screen.getByText('s1')).toBeDefined();
    expect(screen.getByText(/2 msgs/)).toBeDefined();
  });

  it('calls onSelect when item clicked', () => {
    const onSelect = vi.fn();
    const { container } = render(
      <Sidebar
        buckets={sample}
        loading={false}
        error={null}
        activeId={null}
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
        buckets={sample}
        loading={false}
        error={null}
        activeId={null}
        onSelect={vi.fn()}
        onRefresh={onRefresh}
      />,
    );
    const refreshBtn = container.querySelector('button[title="Refresh"]') as HTMLButtonElement;
    refreshBtn.click();
    expect(onRefresh).toHaveBeenCalled();
  });
});