/**
 * Vitest — buttonStyle 工具测试。
 */
import { describe, it, expect } from 'vitest';
import { buttonStyle } from '@/features/design-system/utils/buttonStyles';

describe('buttonStyle', () => {
  it('returns primary colors for primary variant', () => {
    const s = buttonStyle({ variant: 'primary' });
    expect(s.background).toBe('#3b82f6');
    expect(s.color).toBe('#ffffff');
    expect(s.borderColor).toBe('#3b82f6');
  });

  it('returns danger colors for danger variant', () => {
    const s = buttonStyle({ variant: 'danger' });
    expect(s.background).toBe('#ef4444');
  });

  it('ghost has transparent background', () => {
    const s = buttonStyle({ variant: 'ghost' });
    expect(s.background).toBe('transparent');
    expect(s.borderColor).toBe('transparent');
  });

  it('applies sm size padding and font', () => {
    const s = buttonStyle({ variant: 'secondary', size: 'sm' });
    expect(s.padding).toBe('4px 10px');
    expect(s.fontSize).toBe(12);
  });

  it('applies block width when requested', () => {
    const s = buttonStyle({ variant: 'primary', block: true });
    expect(s.width).toBe('100%');
  });

  it('omits width when not block', () => {
    const s = buttonStyle({ variant: 'primary' });
    expect(s.width).toBeUndefined();
  });
});