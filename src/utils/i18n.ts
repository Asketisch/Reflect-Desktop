import React from 'react';
import type { ReactNode } from 'react';
import { createContext, useCallback, useContext, useMemo, useState } from 'react';

export type Locale = 'en' | 'zh-CN';
export const DEFAULT_LOCALE: Locale = 'en';
const STORAGE_KEY = 'reflect.locale';

const STRINGS: Record<string, Record<Locale, string>> = {
  'app.title': { en: 'Reflect Desktop', 'zh-CN': 'Reflect Desktop' },
  'sidebar.sessions': { en: 'Sessions', 'zh-CN': '会话' },
  'sidebar.refresh': { en: 'Refresh', 'zh-CN': '刷新' },
  'sidebar.loading': { en: 'Loading…', 'zh-CN': '加载中…' },
  'sidebar.empty': { en: 'No sessions yet. Start a turn above to create one.', 'zh-CN': '尚无会话。在上方输入第一条消息以创建。' },
  'composer.placeholder': { en: 'Ask Reflect anything…  (type / for commands)', 'zh-CN': '问 Reflect 任何事…(输入 / 查看命令)' },
  'chat.empty.title': { en: 'Start a conversation', 'zh-CN': '开始一次对话' },
  'chat.empty.hint': { en: 'Type below or use a slash command. Press ⌘/Ctrl + Enter to send.', 'zh-CN': '在下方输入或使用 slash 命令。按 ⌘/Ctrl + Enter 发送。' },
  'chat.loading': { en: 'Loading session…', 'zh-CN': '正在加载会话…' },
  'chat.loadError': { en: 'Could not load this session.', 'zh-CN': '无法加载此会话。' },
  'chat.retry': { en: 'Retry', 'zh-CN': '重试' },
  'chat.emptySession': { en: 'This session has no messages.', 'zh-CN': '此会话没有消息。' },
  'chat.viewing': { en: 'Viewing session', 'zh-CN': '正在查看会话' },
  'settings.title': { en: 'Settings', 'zh-CN': '设置' },
  'settings.provider': { en: 'Provider', 'zh-CN': '提供商' },
  'settings.permissions': { en: 'Permissions', 'zh-CN': '权限' },
  'settings.advanced': { en: 'Advanced', 'zh-CN': '高级' },
  'settings.language': { en: 'Language', 'zh-CN': '语言' },
  'settings.english': { en: 'English', 'zh-CN': '英语' },
  'settings.chinese': { en: '简体中文', 'zh-CN': '简体中文' },
  'settings.save': { en: 'Save to ~/.reflect/config.toml', 'zh-CN': '保存到 ~/.reflect/config.toml' },
  'settings.saved': { en: 'Saved ✓', 'zh-CN': '已保存 ✓' },
  'settings.loading': { en: 'Loading status…', 'zh-CN': '正在加载状态…' },
  'settings.activeProvider': { en: 'Active provider', 'zh-CN': '当前提供商' },
  'settings.apiKey': { en: 'API key', 'zh-CN': 'API 密钥' },
  'settings.model': { en: 'Model', 'zh-CN': '模型' },
  'settings.configFields': { en: 'Configuration fields', 'zh-CN': '配置字段' },
  'settings.configHelp': { en: 'Edit supported settings directly. Unknown or advanced values remain available in the TOML editor below.', 'zh-CN': '直接编辑受支持的设置。未知或高级配置仍可在下方 TOML 编辑器中使用。' },
  'common.close': { en: 'Close', 'zh-CN': '关闭' },
  'common.show': { en: 'Show key', 'zh-CN': '显示密钥' },
  'common.hide': { en: 'Hide key', 'zh-CN': '隐藏密钥' },
};

export function detectLocale(): Locale {
  if (typeof window !== 'undefined') {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === 'en' || stored === 'zh-CN') return stored;
    if (navigator.language.toLowerCase().startsWith('zh')) return 'zh-CN';
  }
  return DEFAULT_LOCALE;
}

export function saveLocale(locale: Locale): void {
  if (typeof window !== 'undefined') window.localStorage.setItem(STORAGE_KEY, locale);
}

export function t(key: string, locale: Locale = DEFAULT_LOCALE): string {
  const entry = STRINGS[key];
  return entry?.[locale] ?? entry?.[DEFAULT_LOCALE] ?? key;
}

type I18nContextValue = { locale: Locale; setLocale: (locale: Locale) => void; t: (key: string) => string };
const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);
  const setLocale = useCallback((next: Locale) => { setLocaleState(next); saveLocale(next); }, []);
  const value = useMemo(() => ({ locale, setLocale, t: (key: string) => t(key, locale) }), [locale, setLocale]);
  return React.createElement(I18nContext.Provider, { value }, children);
}

export function useI18n(): I18nContextValue {
  const value = useContext(I18nContext);
  if (!value) return { locale: DEFAULT_LOCALE, setLocale: saveLocale, t: (key) => t(key) };
  return value;
}

export { STRINGS };
