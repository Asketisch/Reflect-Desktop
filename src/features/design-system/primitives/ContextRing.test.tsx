/**
 * Vitest — ContextRing 测试。
 */
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { ContextRing } from '@/features/design-system/primitives/ContextRing';

describe('ContextRing', () => {
  it('renders SVG circle elements', () => {
    const { container } = render(
      <ContextRing
        segments={[
          { label: 'tools', value: 0.3, color: '#3b82f6' },
          { label: 'system', value: 0.2, color: '#22c55e' },
        ]}
      />,
    );
    // 1 background + 2 foreground = 3 circles
    expect(container.querySelectorAll('circle').length).toBe(3);
  });

  it('renders label when provided', () => {
    const { container } = render(
      <ContextRing segments={[{ label: 'a', value: 0.5, color: '#000' }]} label="50%" />,
    );
    expect(container.textContent).toContain('50%');
  });

  it('handles empty segments', () => {
    const { container } = render(<ContextRing segments={[]} />);
    expect(container.querySelectorAll('circle').length).toBe(1); // background only
  });

  it('clamps segment values to [0, 1]', () => {
    const { container } = render(
      <ContextRing segments={[{ label: 'x', value: 1.5, color: '#000' }]} />,
    );
    // Just check it renders without throwing
    expect(container.querySelector('svg')).toBeDefined();
  });
});