/**
 * Vitest — ApprovalHistory.
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ApprovalHistory } from './ApprovalHistory';

describe('ApprovalHistory', () => {
  it('shows empty state when no records', () => {
    render(<ApprovalHistory records={[]} />);
    expect(screen.getByTestId('approval-history-empty')).toBeDefined();
  });

  it('renders records in newest-first order', () => {
    const records = [
      { id: 'a', toolName: 'shell', decision: 'approve' as const, decidedAt: 1000 },
      { id: 'b', toolName: 'write_file', decision: 'deny' as const, decidedAt: 2000, reason: 'risky' },
    ];
    render(<ApprovalHistory records={records} />);
    const rows = screen.getAllByTestId(/^approval-record-/);
    expect(rows[0].getAttribute('data-testid')).toBe('approval-record-b');
    expect(rows[1].getAttribute('data-testid')).toBe('approval-record-a');
  });

  it('respects limit', () => {
    const records = Array.from({ length: 10 }, (_, i) => ({
      id: `r-${i}`,
      toolName: 'shell',
      decision: 'approve' as const,
      decidedAt: i * 1000,
    }));
    render(<ApprovalHistory records={records} limit={3} />);
    expect(screen.getAllByTestId(/^approval-record-/).length).toBe(3);
  });

  it('shows reason when present', () => {
    render(
      <ApprovalHistory
        records={[
          { id: 'a', toolName: 'shell', decision: 'deny', reason: 'too risky', decidedAt: 1000 },
        ]}
      />,
    );
    expect(screen.getByText(/too risky/)).toBeDefined();
  });
});
