/**
 * i18n 工具 —— placeholder for M3.x FormatJS integration.
 *
 * CodexMonitor 同名: `src/utils/i18n.ts`。
 *
 * 当前 M2.x 仅支持 zh-CN / en placeholder；M3.x 完整 i18n 走 `react-intl 10`。
 */

export type Locale = 'zh-CN' | 'en';

export const DEFAULT_LOCALE: Locale = 'zh-CN';

const STRINGS: Record<string, Record<Locale, string>> = {
  'app.title': { 'zh-CN': 'Reflect Desktop', en: 'Reflect Desktop' },
  'sidebar.sessions': { 'zh-CN': '会话', en: 'Sessions' },
  'sidebar.refresh': { 'zh-CN': '刷新', en: 'Refresh' },
  'sidebar.loading': { 'zh-CN': '加载中…', en: 'Loading…' },
  'sidebar.empty': {
    'zh-CN': '尚无会话。在上方输入第一条消息以创建。',
    en: 'No sessions yet. Start a turn above to create one.',
  },
  'thread.empty': { 'zh-CN': '尚无对话。在 Chat 中开始一次对话。', en: 'No threads yet. Start a conversation in Chat.' },
  'settings.title': { 'zh-CN': '设置', en: 'Settings' },
};

export function t(key: string, locale: Locale = DEFAULT_LOCALE): string {
  const entry = STRINGS[key];
  if (!entry) return key;
  return entry[locale] ?? entry[DEFAULT_LOCALE];
}