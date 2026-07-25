/**
 * i18n namespace —— terminal.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const terminal: Record<string, StringEntry> = {
  'terminal.title':         { en: 'Terminal',                                     'zh-CN': '终端' },
  'terminal.new':           { en: 'New terminal',                                 'zh-CN': '新建终端' },
  'terminal.kill':          { en: 'Kill',                                         'zh-CN': '终止' },
  'terminal.clear':         { en: 'Clear',                                        'zh-CN': '清空' },
  'terminal.copy':          { en: 'Copy',                                         'zh-CN': '复制' },
  'terminal.paste':         { en: 'Paste',                                        'zh-CN': '粘贴' },
  'terminal.sessionCount':  { en: '{count} session{plural}',                      'zh-CN': '{count} 个会话' },
  'terminal.sessionsAria':  { en: 'Shell sessions',                               'zh-CN': 'Shell 会话' },
  'terminal.outputAria':    { en: 'Terminal output',                              'zh-CN': '终端输出' },
  'terminal.empty.title':   { en: 'No active sessions',                           'zh-CN': '没有活跃会话' },
  'terminal.empty.hint':    { en: 'Type a shell command below and press Enter.',  'zh-CN': '在下方输入 shell 命令并按 Enter。' },
  'terminal.exit':          { en: 'exit',                                         'zh-CN': '退出码' },
  'terminal.cwd':           { en: 'cwd',                                          'zh-CN': '工作目录' },
};

export default terminal;
