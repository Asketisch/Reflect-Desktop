/**
 * i18n namespace —— threads.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const threads: Record<string, StringEntry> = {
  'threads.title':              { en: 'Threads',                             'zh-CN': '会话' },
  'threads.subtitle':           { en: 'All sessions across the workspace.',  'zh-CN': '工作区中的所有会话。' },
  'threads.empty':              { en: 'No threads yet.',                     'zh-CN': '暂无会话。' },
  'threads.searchPlaceholder':  { en: 'Search threads…',                     'zh-CN': '搜索会话…' },
};

export default threads;
