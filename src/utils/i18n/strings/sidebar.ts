/**
 * i18n namespace —— sidebar.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const sidebar: Record<string, StringEntry> = {
  'sidebar.sessions':           { en: 'Sessions',                                            'zh-CN': '会话' },
  'sidebar.refresh':            { en: 'Refresh',                                             'zh-CN': '刷新' },
  'sidebar.loading':            { en: 'Loading…',                                            'zh-CN': '加载中…' },
  'sidebar.empty':              { en: 'No sessions yet. Start a turn above to create one.',  'zh-CN': '暂无会话。在上方输入第一条消息以创建。' },
  'sidebar.newChat':            { en: 'New chat',                                            'zh-CN': '新建对话' },
  'sidebar.searchPlaceholder':  { en: 'Search sessions…',                                    'zh-CN': '搜索会话…' },
  'sidebar.searchAriaLabel':    { en: 'Search sessions',                                     'zh-CN': '搜索会话' },
  'sidebar.bucket.now':         { en: 'Now',                                                 'zh-CN': '现在' },
  'sidebar.bucket.today':       { en: 'Today',                                               'zh-CN': '今天' },
  'sidebar.bucket.yesterday':   { en: 'Yesterday',                                           'zh-CN': '昨天' },
  'sidebar.bucket.thisWeek':    { en: 'This Week',                                           'zh-CN': '本周' },
  'sidebar.bucket.older':       { en: 'Older',                                               'zh-CN': '更早' },
  'sidebar.untitled':           { en: 'Untitled session',                                    'zh-CN': '未命名会话' },
  'sidebar.deleteConfirm':      { en: 'Delete this session?',                                'zh-CN': '删除此会话?' },
  'sidebar.renamePrompt':       { en: 'New session name',                                    'zh-CN': '新会话名称' },
};

export default sidebar;
