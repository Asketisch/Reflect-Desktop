/**
 * i18n 命名空间 —— composer.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const composer: Record<string, StringEntry> = {
  'composer.placeholder':                    { en: 'Ask Reflect anything…  (type / for commands)',  'zh-CN': '向 Reflect 提问…(输入 / 查看命令)' },
  'composer.ariaLabel':                      { en: 'Message Reflect',                               'zh-CN': '向 Reflect 发送消息' },
  'composer.toolbar.slashCommands':          { en: 'Slash commands',                                'zh-CN': 'Slash 命令' },
  'composer.toolbar.attachFile':             { en: 'Attach file',                                   'zh-CN': '附加文件' },
  'composer.toolbar.attachImage':            { en: 'Attach image',                                  'zh-CN': '附加图片' },
  'composer.toolbar.mentionSkill':           { en: 'Mention skill',                                 'zh-CN': '提及技能' },
  'composer.toolbar.mentionFile':            { en: 'Mention file',                                  'zh-CN': '提及文件' },
  'composer.sendHintMac':                    { en: 'Send (Enter) · New line (Shift+Enter)',        'zh-CN': '发送 (Enter) · 换行 (Shift+Enter)' },
  'composer.sendHintOther':                  { en: 'Send (Enter) · New line (Shift+Enter)',        'zh-CN': '发送 (Enter) · 换行 (Shift+Enter)' },
  'composer.compactRequested':               { en: 'Compact requested.',                            'zh-CN': '已请求压缩上下文。' },
  'composer.exported':                       { en: 'Exported → {path}',                             'zh-CN': '已导出 → {path}' },
  'composer.exportedNoPath':                 { en: 'Exported → (no path)',                          'zh-CN': '已导出 → (无路径)' },
  'composer.unhandledSlash':                 { en: 'Unhandled slash submission: {kind}',            'zh-CN': '未处理的 slash 提交: {kind}' },
  'composer.commandRejected':                { en: 'Command rejected.',                             'zh-CN': '命令被拒绝。' },
  'composer.commandHandled':                 { en: 'Command handled.',                              'zh-CN': '命令已处理。' },
  'composer.attachmentBar.remove':           { en: 'Remove attachment',                             'zh-CN': '移除附件' },
  'composer.attachmentBar.addLocalImage':    { en: 'Image:',                                        'zh-CN': '图片:' },
  'composer.attachmentBar.addInlineImage':   { en: 'Inline image',                                  'zh-CN': '内嵌图片' },
  'composer.attachmentBar.addFile':          { en: 'File:',                                         'zh-CN': '文件:' },
  'composer.attachmentBar.addSkillMention':  { en: 'Skill:',                                        'zh-CN': '技能:' },
  'composer.mentionPicker.ariaLabel':        { en: 'Files',                                         'zh-CN': '文件' },
  'composer.mentionPicker.loading':          { en: 'Loading files…',                                'zh-CN': '正在加载文件…' },
  'composer.mentionPicker.empty':            { en: 'No matching files.',                            'zh-CN': '没有匹配的文件。' },
  'composer.slashPopup.title':               { en: 'Slash commands',                                'zh-CN': 'Slash 命令' },
  'composer.slashPopup.empty':               { en: 'No matching commands',                          'zh-CN': '没有匹配的命令' },
  'composer.tooManyAttachments':             { en: 'Too many attachments (max {max}).',             'zh-CN': '附件过多 (最多 {max} 个)。' },
};

export default composer;
