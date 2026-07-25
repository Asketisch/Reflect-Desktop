/**
 * i18n namespace —— workspaces.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const workspaces: Record<string, StringEntry> = {
  'workspaces.title':         { en: 'Workspaces',                                                                                                                  'zh-CN': '工作区' },
  'workspaces.subtitle':      { en: 'Switch projects and configure paths.',                                                                                        'zh-CN': '切换项目并配置路径。' },
  'workspaces.current':       { en: 'Current workspace',                                                                                                           'zh-CN': '当前工作区' },
  'workspaces.switch':        { en: 'Switch',                                                                                                                      'zh-CN': '切换' },
  'workspaces.add':           { en: 'Add workspace',                                                                                                               'zh-CN': '添加工作区' },
  'workspaces.remove':        { en: 'Remove workspace',                                                                                                            'zh-CN': '移除工作区' },
  'workspaces.empty':         { en: 'No workspaces configured.',                                                                                                   'zh-CN': '未配置工作区。' },
  'workspaces.recent':        { en: 'Recent workspaces',                                                                                                           'zh-CN': '最近工作区' },
  'workspaces.note':          { en: 'Switching workspaces here sets the active path for new sessions. Sessions already in flight remain on their original path.',  'zh-CN': '切换工作区会为新会话设置当前路径。已在运行中的会话保留其原路径。' },
  'workspaces.emptyDesc':     { en: 'Start a chat to record your first workspace.',                                                                                'zh-CN': '开启对话来记录你的第一个工作区。' },
  'workspaces.setTo':         { en: 'Workspace set to {name}',                                                                                                     'zh-CN': '工作区已切换到 {name}' },
  'workspaces.switchFailed':  { en: 'Failed to switch workspace: {msg}',                                                                                           'zh-CN': '切换工作区失败: {msg}' },
  'workspaces.use':           { en: 'Use',                                                                                                                         'zh-CN': '使用' },
  'workspaces.sessionCount':  { en: '{count} session{plural}',                                                                                                     'zh-CN': '{count} 个会话' },
  'workspaces.unknown':       { en: '(unknown)',                                                                                                                   'zh-CN': '(未知)' },
};

export default workspaces;
