/**
 * Vitest — i18n 测试。
 */
import { describe, it, expect } from 'vitest';
import { t, DEFAULT_LOCALE } from '@/utils/i18n';

describe('t()', () => {
  it('returns zh-CN by default', () => {
    expect(t('app.title')).toBe('Reflect Desktop');
    expect(t('sidebar.sessions')).toBe('会话');
  });

  it('returns English when locale=en', () => {
    expect(t('sidebar.sessions', 'en')).toBe('Sessions');
    expect(t('sidebar.loading', 'en')).toBe('Loading…');
  });

  it('returns key as fallback for unknown keys', () => {
    expect(t('nonexistent.key', 'en')).toBe('nonexistent.key');
  });

  it('exposes DEFAULT_LOCALE', () => {
    expect(DEFAULT_LOCALE).toBe('zh-CN');
  });
});