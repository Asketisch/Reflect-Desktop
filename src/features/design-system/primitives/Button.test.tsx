/**
 * Vitest — Button primitive 测试。
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button } from '@/features/design-system/primitives/Button';

describe('Button', () => {
  it('renders text', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).toBeDefined();
  });

  it('defaults to type=button', () => {
    render(<Button>x</Button>);
    expect(screen.getByText('x').getAttribute('type')).toBe('button');
  });

  it('accepts submit type', () => {
    render(<Button type="submit">Submit</Button>);
    const btn = screen.getByText('Submit');
    expect(btn.getAttribute('type')).toBe('submit');
  });

  it('forwards onClick', () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Action</Button>);
    screen.getByText('Action').click();
    expect(onClick).toHaveBeenCalled();
  });

  it('renders primary variant (background in style)', () => {
    const { container } = render(<Button variant="primary">P</Button>);
    const btn = container.querySelector('button')!;
    expect(btn.style.background).toContain('59, 130, 246'); // rgb form
  });

  it('renders danger variant (background in style)', () => {
    const { container } = render(<Button variant="danger">D</Button>);
    const btn = container.querySelector('button')!;
    expect(btn.style.background).toContain('239, 68, 68');
  });

  it('respects disabled', () => {
    const { container } = render(<Button disabled>x</Button>);
    expect((container.querySelector('button') as HTMLButtonElement).disabled).toBe(true);
  });
});