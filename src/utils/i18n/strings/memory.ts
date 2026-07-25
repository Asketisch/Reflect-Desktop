/**
 * i18n namespace —— memory.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const memory: Record<string, StringEntry> = {
  'memory.title':          { en: 'Memory',                    'zh-CN': '记忆' },
  'memory.subtitle':       { en: 'Persistent agent memory.',  'zh-CN': '持久化的 agent 记忆。' },
  'memory.scope.all':      { en: 'All scopes',                'zh-CN': '所有范围' },
  'memory.scope.user':     { en: 'User',                      'zh-CN': '用户' },
  'memory.scope.project':  { en: 'Project',                   'zh-CN': '项目' },
  'memory.scope.session':  { en: 'Session',                   'zh-CN': '会话' },
  'memory.empty':          { en: 'No memory entries.',        'zh-CN': '暂无记忆。' },
  'memory.add':            { en: 'Add memory',                'zh-CN': '添加记忆' },
  'memory.key':            { en: 'Key',                       'zh-CN': '键' },
  'memory.value':          { en: 'Value',                     'zh-CN': '值' },
  'memory.scope':          { en: 'Scope',                     'zh-CN': '范围' },
};

export default memory;
