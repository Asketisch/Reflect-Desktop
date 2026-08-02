/**
 * i18n namespace —— apps.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const apps: Record<string, StringEntry> = {
  'apps.title':        { en: 'Apps',                             'zh-CN': '应用' },
  'apps.subtitle':     { en: 'Connect IDEs and external tools.', 'zh-CN': '连接 IDE 和外部工具。' },
  'apps.empty':        { en: 'No apps loaded.',                  'zh-CN': '暂无应用。' },
  'apps.connected':    { en: 'connected',                        'zh-CN': '已连接' },
  'apps.notConnected': { en: 'Not connected',                    'zh-CN': '未连接' },
  'apps.connect':      { en: 'Connect',                          'zh-CN': '连接' },
  'apps.disconnect':   { en: 'Disconnect',                       'zh-CN': '断开' },
};

export default apps;