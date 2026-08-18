/**
 * Vitest — uuid 工具测试。
 */
import { describe, it, expect } from 'vitest';
import { uuid, shortId } from '@/utils/uuid';

describe('uuid', () => {
  it('generates unique values', () => {
    const a = uuid();
    const b = uuid();
    expect(a).not.toBe(b);
    expect(a.length).toBeGreaterThan(0);
  });

  it('uses crypto.randomUUID when available', () => {
    const id = uuid();
    // crypto.randomUUID 返回带连字符的 36 字符 UUID
    expect(id.length).toBeGreaterThanOrEqual(36);
  });
});

describe('shortId', () => {
  it('takes first 8 chars', () => {
    expect(shortId('abcdef12-3456-7890')).toBe('abcdef12');
  });

  it('returns full id if shorter than 8', () => {
    expect(shortId('abc')).toBe('abc');
  });
});