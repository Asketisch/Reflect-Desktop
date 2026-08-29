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
  'chat.diffLabel':                { en: 'file change',                                                       'zh-CN': '文件改动' },
  'chat.greeting.morning':         { en: 'Good morning',                                                      'zh-CN': '早上好' },
  'chat.greeting.afternoon':       { en: 'Good afternoon',                                                    'zh-CN': '下午好' },
  'chat.greeting.evening':         { en: 'Good evening',                                                      'zh-CN': '晚上好' },
  'chat.greeting.subtitle':        { en: 'What can I help you with?',                                         'zh-CN': '有什么想让我帮忙的吗？' },
  'chat.contextWarn':              { en: 'Context window is {pct}% full — consider compacting to keep the session fast.',  'zh-CN': '上下文已用 {pct}%，建议压缩以保持会话流畅。' },
  'chat.contextCompactNow':        { en: 'Compact now',                                                       'zh-CN': '立即压缩' },
  'chat.hero.workspace':           { en: 'Workspace: {name}',                                                 'zh-CN': '当前工作区：{name}' },
  'chat.quick.title':              { en: 'Try one of these',                                                  'zh-CN': '试试这些' },
  'chat.quick.explore.title':      { en: 'Explore the code',                                                  'zh-CN': '探索并理解代码' },
  'chat.quick.explore.prompt':     { en: 'Explore this codebase: summarize its overall structure, key modules and how they connect.',        'zh-CN': '探索这个代码库：总结整体结构、关键模块以及它们之间的联系。' },
  'chat.quick.build.title':        { en: 'Build a feature',                                                   'zh-CN': '构建新功能' },
  'chat.quick.build.prompt':       { en: 'Help me implement a new feature: ',                                  'zh-CN': '帮我实现一个新功能：' },
  'chat.quick.review.title':       { en: 'Review changes',                                                    'zh-CN': '审查代码改动' },
  'chat.quick.review.prompt':      { en: 'Review the recent changes in this workspace and suggest improvements.',                            'zh-CN': '审查当前工作区的最近改动，并提出修改建议。' },
  'chat.quick.fix.title':          { en: 'Fix a problem',                                                     'zh-CN': '修复问题' },
  'chat.quick.fix.prompt':         { en: 'Fix the following problem or failing test: ',                        'zh-CN': '修复以下问题或失败的测试：' },
};

export default chat;
