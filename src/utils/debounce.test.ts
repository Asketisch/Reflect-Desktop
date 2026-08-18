/**
 * Vitest — debounce / throttle 测试。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { debounce, throttle } from '@/utils/debounce';

describe('debounce', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('coalesces rapid calls into one', () => {
    const fn = vi.fn();
    const d = debounce(fn, 100);
    d('a');
    d('b');
    d('c');
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('c'); // last args win
  });

  it('resets timer on each call', () => {
    const fn = vi.fn();
    const d = debounce(fn, 100);
    d('x');
    vi.advanceTimersByTime(80);
    d('y');
    vi.advanceTimersByTime(80);
    expect(fn).not.toHaveBeenCalled(); // 80 + 80 = 160，但 80 处已重置
    vi.advanceTimersByTime(20);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('y');
  });
});

describe('throttle', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('fires immediately on first call', () => {
    const fn = vi.fn();
    const t = throttle(fn, 100);
    t('a');
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith('a');
  });

  it('coalesces within window', () => {
    const fn = vi.fn();
    const t = throttle(fn, 100);
    t('a');
    t('b');
    t('c');
    expect(fn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(2);
    expect(fn).toHaveBeenLastCalledWith('c');
  });
});