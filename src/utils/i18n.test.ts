/**
 * Vitest — i18n 测试。
 */
import { describe, it, expect } from 'vitest';
import { t, DEFAULT_LOCALE, type Locale } from '@/utils/i18n';

describe('t()', () => {
  it('DEFAULT_LOCALE is en (UI 主体语言)', () => {
    expect(DEFAULT_LOCALE).toBe('en');
  });

  it('returns English by default', () => {
    expect(t('app.title')).toBe('Reflect Desktop');
    expect(t('sidebar.sessions')).toBe('Sessions');
  });

  it('returns zh-CN when explicitly requested', () => {
    const zh: Locale = 'zh-CN';
    expect(t('sidebar.sessions', zh)).toBe('会话');
    expect(t('sidebar.refresh', zh)).toBe('刷新');
  });

  it('returns English when locale=en', () => {
    expect(t('sidebar.sessions', 'en')).toBe('Sessions');
    expect(t('sidebar.loading', 'en')).toBe('Loading…');
  });

  it('returns key as fallback for unknown keys', () => {
    expect(t('nonexistent.key', 'en')).toBe('nonexistent.key');
  });

  it('composer/chat empty strings are present in both locales', () => {
    expect(t('composer.placeholder', 'en')).toContain('Ask Reflect');
    expect(t('composer.placeholder', 'zh-CN')).toContain('Reflect');
    expect(t('chat.empty.title', 'en')).toBe('Start a conversation');
    expect(t('chat.empty.title', 'zh-CN')).toBe('开始一次对话');
  });
});