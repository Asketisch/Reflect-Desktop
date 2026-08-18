/**
 * i18n 命名空间 —— notifications.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const notifications: Record<string, StringEntry> = {
  'notifications.title':     { en: 'Notifications',                'zh-CN': '通知' },
  'notifications.subtitle':  { en: 'Recent agent notifications.',  'zh-CN': '最近的 agent 通知。' },
  'notifications.empty':     { en: 'No notifications.',            'zh-CN': '暂无通知。' },
  'notifications.markRead':  { en: 'Mark all read',                'zh-CN': '全部标记为已读' },
  'notifications.clear':     { en: 'Clear',                        'zh-CN': '清空' },
};

export default notifications;
