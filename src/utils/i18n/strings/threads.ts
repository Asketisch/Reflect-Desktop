/**
 * i18n namespace —— threads.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const threads: Record<string, StringEntry> = {
  'threads.title':              { en: 'Threads',                              'zh-CN': '会话' },
  'threads.subtitle':           { en: 'All sessions across the workspace.',   'zh-CN': '工作区中的所有会话。' },
  'threads.empty':              { en: 'No threads yet.',                      'zh-CN': '暂无会话。' },
  'threads.searchPlaceholder':  { en: 'Search threads…',                      'zh-CN': '搜索会话…' },
  'threads.msgCount':           { en: '{count} message{plural}',              'zh-CN': '{count} 条消息' },
  'threads.rename':             { en: 'Rename',                               'zh-CN': '重命名' },
  'threads.export':             { en: 'Export',                               'zh-CN': '导出' },
  'threads.delete':             { en: 'Delete',                               'zh-CN': '删除' },
  'threads.cancel':             { en: 'Cancel',                               'zh-CN': '取消' },
  'threads.save':               { en: 'Save',                                 'zh-CN': '保存' },
  'threads.saving':             { en: 'Saving…',                              'zh-CN': '保存中…' },
  'threads.threadName':         { en: 'Thread name',                          'zh-CN': '会话名称' },
  'threads.deleteConfirm':      { en: 'Delete thread "{id}"?\nThis removes its rollout files permanently.', 'zh-CN': '删除会话 "{id}"？\n这将永久移除其 rollout 文件。' },
  'threads.exported':           { en: 'Exported → {path}',                    'zh-CN': '已导出 → {path}' },
  'threads.threadActions':      { en: 'Thread actions',                       'zh-CN': '会话操作' },
  'threads.renameThread':       { en: 'Rename thread',                        'zh-CN': '重命名会话' },
};

export default threads;