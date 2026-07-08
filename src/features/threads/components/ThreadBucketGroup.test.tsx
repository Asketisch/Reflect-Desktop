/**
 * Vitest — ThreadBucketGroup 测试。
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ThreadBucketGroup } from '@/features/threads/components/ThreadBucketGroup';
import type { SessionBucket } from '@/features/sessions';

vi.mock('@tanstack/react-router', async () => {
  const actual = await vi.importActual('@tanstack/react-router');
  return {
    ...actual,
    Link: ({
      children,
      to,
      onClick,
      style,
      ...rest
    }: {
      children: React.ReactNode;
      to: string;
      onClick?: () => void;
      style?: React.CSSProperties;
      [k: string]: unknown;
    }) => (
      <a href={to} onClick={onClick} style={style} {...rest}>
        {children}
      </a>
    ),
  };
});

const NOW = Date.now();

function bucket(): SessionBucket {
  return {
    label: 'Now',
    sessions: [
      {
        session_id: 's1',
        thread_id: 't1',
        model: 'stub/test',
        provider: 'local',
        started_at: new Date(NOW - 30 * 60 * 1000).toISOString(),
        message_count: 2,
        tool_count: 0,
        token_total: 0,
        cwd: '/tmp',
        display_name: 'Recent chat',
      },
    ],
  };
}

describe('ThreadBucketGroup', () => {
  it('renders bucket label and session', () => {
    render(<ThreadBucketGroup bucket={bucket()} activeId={null} onSelect={vi.fn()} />);
    expect(screen.getByText('Now')).toBeDefined();
    expect(screen.getByText('Recent chat')).toBeDefined();
    expect(screen.getByText(/2 msgs/)).toBeDefined();
  });

  it('marks active item', () => {
    const { container } = render(<ThreadBucketGroup bucket={bucket()} activeId="s1" onSelect={vi.fn()} />);
    const link = container.querySelector('a[data-active="true"]');
    expect(link).not.toBeNull();
  });

  it('renders bucket label and session content', () => {
    const { container } = render(<ThreadBucketGroup bucket={bucket()} activeId={null} onSelect={vi.fn()} />);
    expect(container.querySelector('[data-bucket="Now"]')).not.toBeNull();
    expect(container.textContent).toContain('Recent chat');
    expect(container.textContent).toContain('2 msgs');
  });

  it('calls onSelect when clicked', () => {
    const onSelect = vi.fn();
    const { container } = render(<ThreadBucketGroup bucket={bucket()} activeId={null} onSelect={onSelect} />);
    const link = container.querySelector('a')!;
    link.click();
    expect(onSelect).toHaveBeenCalledWith('s1');
  });

  it('renders multiple sessions', () => {
    const b: SessionBucket = {
      label: 'Today',
      sessions: [
        { ...bucket().sessions[0], session_id: 's1', display_name: 'Chat A' },
        { ...bucket().sessions[0], session_id: 's2', display_name: 'Chat B' },
      ],
    };
    render(<ThreadBucketGroup bucket={b} activeId={null} onSelect={vi.fn()} />);
    expect(screen.getByText('Chat A')).toBeDefined();
    expect(screen.getByText('Chat B')).toBeDefined();
  });
});