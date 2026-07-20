/**
 * i18n 工具 —— placeholder for M3.x FormatJS integration.
 *
 * CodexMonitor 同名: `src/utils/i18n.ts`。
 *
 * 当前默认 locale = 'en'（UI 主体语言）；'zh-CN' 为翻译副本。完整 i18n 走 react-intl 10。
 */

export type Locale = 'en' | 'zh-CN';

export const DEFAULT_LOCALE: Locale = 'en';

const STRINGS: Record<string, Record<Locale, string>> = {
  'app.title': { en: 'Reflect Desktop', 'zh-CN': 'Reflect Desktop' },
  'sidebar.sessions': { en: 'Sessions', 'zh-CN': '会话' },
  'sidebar.refresh': { en: 'Refresh', 'zh-CN': '刷新' },
  'sidebar.loading': { en: 'Loading…', 'zh-CN': '加载中…' },
  'sidebar.empty': {
    en: 'No sessions yet. Start a turn above to create one.',
    'zh-CN': '尚无会话。在上方输入第一条消息以创建。',
  },
  'thread.empty': { en: 'No threads yet. Start a conversation in Chat.', 'zh-CN': '尚无对话。在 Chat 中开始一次对话。' },
  'settings.title': { en: 'Settings', 'zh-CN': '设置' },
  'composer.placeholder': {
    en: 'Ask Reflect anything…  (type / for commands)',
    'zh-CN': '问 Reflect 任何事…(输入 / 查看命令)',
  },
  'chat.empty.title': { en: 'Start a conversation', 'zh-CN': '开始一次对话' },
  'chat.empty.hint': {
    en: 'Type below or use a slash command. Press ⌘/Ctrl + Enter to send.',
    'zh-CN': '在下方输入或使用 slash 命令。按 ⌘/Ctrl + Enter 发送。',
  },
};

export function t(key: string, locale: Locale = DEFAULT_LOCALE): string {
  const entry = STRINGS[key];
  if (!entry) return key;
  return entry[locale] ?? entry[DEFAULT_LOCALE];
}