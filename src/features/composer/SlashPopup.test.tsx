/**
 * Vitest — SlashPopup 测试。
 */
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';
import { SlashPopup } from '@/features/composer/SlashPopup';

describe('SlashPopup', () => {
  it('returns null when not visible', () => {
    const { container } = render(
      <SlashPopup query="" onSelect={vi.fn()} visible={false} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('returns null when no matches', () => {
    const { container } = render(
      <SlashPopup query="zzzzzz" onSelect={vi.fn()} visible={true} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('lists all commands when query is empty', () => {
    const { container } = render(<SlashPopup query="" onSelect={vi.fn()} visible={true} />);
    expect(container.textContent).toContain('/theme');
    expect(container.textContent).toContain('/vim');
  });

  it('filters by name prefix', () => {
    const { container } = render(<SlashPopup query="eff" onSelect={vi.fn()} visible={true} />);
    expect(container.textContent).toContain('/effort');
    expect(container.textContent).not.toContain('/theme');
  });

  it('filters by alias prefix', () => {
    const { container } = render(<SlashPopup query="?" onSelect={vi.fn()} visible={true} />);
    // "?" is alias for "help"
    expect(container.textContent).toContain('/help');
  });

  it('calls onSelect when command clicked', () => {
    const onSelect = vi.fn();
    const { container } = render(<SlashPopup query="theme" onSelect={onSelect} visible={true} />);
    const btn = Array.from(container.querySelectorAll('button')).find(
      (b) => b.textContent?.includes('/theme'),
    );
    expect(btn).toBeDefined();
    btn!.click();
    expect(onSelect).toHaveBeenCalledWith('theme');
  });

  it('updates active index on mouse enter (no throw)', () => {
    const { container } = render(<SlashPopup query="" onSelect={vi.fn()} visible={true} />);
    const buttons = container.querySelectorAll('button');
    expect(buttons.length).toBeGreaterThan(0);
    buttons[3]?.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
  });
});