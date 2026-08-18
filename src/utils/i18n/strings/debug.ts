/**
 * i18n 命名空间 —— debug.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const debug: Record<string, StringEntry> = {
  'debug.title':     { en: 'Debug',                                    'zh-CN': '调试' },
  'debug.subtitle':  { en: 'Agent state, store snapshot, event log.',  'zh-CN': 'Agent 状态、store 快照、事件日志。' },
  'debug.refresh':   { en: 'Refresh',                                  'zh-CN': '刷新' },
  'debug.copy':      { en: 'Copy state',                               'zh-CN': '复制状态' },
  'debug.clear':     { en: 'Clear log',                                'zh-CN': '清空日志' },
};

export default debug;
