/**
 * Vitest — time 工具测试。
 */
import { describe, it, expect } from 'vitest';
import { relativeTime, clockTime, formatDuration } from '@/utils/time';

const NOW = new Date('2026-07-08T12:00:00Z').getTime();

describe('relativeTime', () => {
  it('returns "just now" for <60s past', () => {
    expect(relativeTime(new Date(NOW - 30_000).toISOString(), NOW)).toBe('just now');
  });

  it('returns minutes for <60m', () => {
    expect(relativeTime(new Date(NOW - 5 * 60_000).toISOString(), NOW)).toBe('5 min ago');
  });

  it('returns hours for <24h', () => {
    expect(relativeTime(new Date(NOW - 3 * 3600_000).toISOString(), NOW)).toBe('3 h ago');
  });

  it('returns days for <7d', () => {
    expect(relativeTime(new Date(NOW - 3 * 86400_000).toISOString(), NOW)).toBe('3 d ago');
  });

  it('returns weeks for <4w', () => {
    expect(relativeTime(new Date(NOW - 14 * 86400_000).toISOString(), NOW)).toBe('2 w ago');
  });

  it('returns months for <12mo', () => {
    expect(relativeTime(new Date(NOW - 90 * 86400_000).toISOString(), NOW)).toBe('3 mo ago');
  });

  it('returns years for older', () => {
    expect(relativeTime(new Date(NOW - 400 * 86400_000).toISOString(), NOW)).toBe('1 y ago');
  });

  it('handles future times', () => {
    expect(relativeTime(new Date(NOW + 30_000).toISOString(), NOW)).toBe('in a moment');
    expect(relativeTime(new Date(NOW + 5 * 60_000).toISOString(), NOW)).toBe('in 5 min');
  });

  it('returns empty for invalid input', () => {
    expect(relativeTime('not-a-date', NOW)).toBe('');
  });
});

describe('clockTime', () => {
  it('formats HH:MM:SS', () => {
    const d = new Date('2026-01-01T09:05:07Z');
    // 依赖 locale；仅检查返回长度为 8 的非空字符串
    const out = clockTime(d);
    expect(out).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });

  it('accepts timestamp number', () => {
    const out = clockTime(0);
    expect(out).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });
});

describe('formatDuration', () => {
  it('formats sub-second as ms', () => {
    expect(formatDuration(150)).toBe('150ms');
  });

  it('formats seconds', () => {
    expect(formatDuration(1500)).toBe('1.5s');
  });

  it('formats minutes + seconds', () => {
    expect(formatDuration(125_000)).toBe('2m 5s');
  });
});