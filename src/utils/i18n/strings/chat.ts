/**
 * i18n 命名空间 —— chat.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const chat: Record<string, StringEntry> = {
  'chat.empty.title':              { en: 'Start a conversation',                                              'zh-CN': '开始一次对话' },
  'chat.empty.hint':               { en: 'Type below or use a slash command. Press ⌘/Ctrl + Enter to send.',  'zh-CN': '在下方输入或使用 slash 命令。按 ⌘/Ctrl + Enter 发送。' },
  'chat.loading':                  { en: 'Loading session…',                                                  'zh-CN': '正在加载会话…' },
  'chat.loadError':                { en: 'Could not load this session.',                                      'zh-CN': '无法加载此会话。' },
  'chat.loadErrorWithMsg':         { en: 'Could not load this session: {msg}',                                'zh-CN': '无法加载此会话: {msg}' },
  'chat.retry':                    { en: 'Retry',                                                             'zh-CN': '重试' },
  'chat.emptySession':             { en: 'This session has no messages.',                                     'zh-CN': '此会话没有消息。' },
  'chat.viewing':                  { en: 'Viewing session',                                                   'zh-CN': '正在查看会话' },
  'chat.thinking':                 { en: 'thinking',                                                          'zh-CN': '思考中' },
  'chat.thinkingWith':             { en: 'thinking: {snippet}',                                               'zh-CN': '思考中: {snippet}' },
  'chat.output':                   { en: 'output',                                                            'zh-CN': '输出' },
  'chat.outputError':              { en: 'output (error)',                                                    'zh-CN': '输出 (错误)' },
  'chat.compacted':                { en: 'Context compacted',                                                 'zh-CN': '上下文已压缩' },
  'chat.tool.running':             { en: 'running',                                                           'zh-CN': '运行中' },
  'chat.tool.success':             { en: 'success',                                                           'zh-CN': '成功' },
  'chat.tool.error':               { en: 'error',                                                             'zh-CN': '失败' },
  'chat.tool.permission':          { en: 'permission',                                                        'zh-CN': '权限' },
  'chat.tool.permissionRequired':  { en: 'Permission required',                                               'zh-CN': '需要权限' },
  'chat.tool.allow':               { en: 'Allow',                                                             'zh-CN': '允许' },
  'chat.tool.deny':                { en: 'Deny',                                                              'zh-CN': '拒绝' },
  'chat.tool.args':                { en: 'args',                                                              'zh-CN': '参数' },
  'chat.tool.emptyArgs':           { en: '(no args)',                                                         'zh-CN': '(无参数)' },
  'chat.tool.noPath':              { en: '(no path)',                                                         'zh-CN': '(无路径)' },
  'chat.tool.chars':               { en: '{n} chars',                                                         'zh-CN': '{n} 字符' },
  'chat.tool.edit':                { en: 'edit',                                                              'zh-CN': '编辑' },
  'chat.tool.create':              { en: 'create',                                                            'zh-CN': '创建' },
  'chat.ariaLabel':                { en: 'Conversation',                                                      'zh-CN': '对话' },
  'chat.diffPanel':                { en: 'Git diff panel',                                                    'zh-CN': 'Git diff 面板' },
  'chat.diffHeader':               { en: 'Working tree diff',                                                 'zh-CN': '工作区 diff' },
  'chat.diffLoading':              { en: 'Loading diff…',                                                     'zh-CN': '正在加载 diff…' },
  'chat.diffEmpty':                { en: 'No changes.',                                                       'zh-CN': '没有改动。' },
};

export default chat;
