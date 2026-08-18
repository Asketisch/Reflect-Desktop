/**
 * i18n 命名空间 —— permissionMode.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const permissionMode: Record<string, StringEntry> = {
  'permissionMode.auto.name':     { en: 'auto',                                             'zh-CN': 'auto' },
  'permissionMode.auto.desc':     { en: 'Run tools without asking. Fastest, least safe.',   'zh-CN': '运行工具时无需询问。速度最快,安全性最低。' },
  'permissionMode.prompt.name':   { en: 'prompt',                                           'zh-CN': 'prompt' },
  'permissionMode.prompt.desc':   { en: 'Ask before each tool call. Recommended.',          'zh-CN': '每次调用工具前询问。推荐设置。' },
  'permissionMode.deny.name':     { en: 'deny',                                             'zh-CN': 'deny' },
  'permissionMode.deny.desc':     { en: 'Block all tool execution. Read-only chat.',        'zh-CN': '禁止所有工具执行。只读对话。' },
  'permissionMode.plan.name':     { en: 'plan',                                             'zh-CN': 'plan' },
  'permissionMode.plan.desc':     { en: 'Only plan, never execute. Explore safely.',        'zh-CN': '仅规划,从不执行。安全探索。' },
  'permissionMode.accept_edits.name': { en: 'accept_edits',                                   'zh-CN': 'accept_edits' },
  'permissionMode.accept_edits.desc': { en: 'Auto-approve file edits/writes. Other tools still require approval.', 'zh-CN': '自动批准文件编辑/写入。其他工具仍需审批。' },
  'permissionMode.bubble.name':     { en: 'bubble',                                         'zh-CN': 'bubble' },
  'permissionMode.bubble.desc':      { en: 'Show permission prompts inline as bubbles in the chat.', 'zh-CN': '在对话中以内联气泡形式显示权限提示。' },
  'permissionMode.bypass.name':     { en: 'bypass',                                         'zh-CN': 'bypass' },
  'permissionMode.bypass.desc':      { en: 'Bypass all permission checks. Use with extreme caution.', 'zh-CN': '绕过所有权限检查。极其谨慎使用。' },
};

export default permissionMode;
