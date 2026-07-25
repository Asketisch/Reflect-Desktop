/**
 * i18n namespace —— permissionMode.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const permissionMode: Record<string, StringEntry> = {
  'permissionMode.auto.name':    { en: 'auto',                                            'zh-CN': 'auto' },
  'permissionMode.auto.desc':    { en: 'Run tools without asking. Fastest, least safe.',  'zh-CN': '运行工具时无需询问。速度最快,安全性最低。' },
  'permissionMode.prompt.name':  { en: 'prompt',                                          'zh-CN': 'prompt' },
  'permissionMode.prompt.desc':  { en: 'Ask before each tool call. Recommended.',         'zh-CN': '每次调用工具前询问。推荐设置。' },
  'permissionMode.deny.name':    { en: 'deny',                                            'zh-CN': 'deny' },
  'permissionMode.deny.desc':    { en: 'Block all tool execution. Read-only chat.',       'zh-CN': '禁止所有工具执行。只读对话。' },
  'permissionMode.plan.name':    { en: 'plan',                                            'zh-CN': 'plan' },
  'permissionMode.plan.desc':    { en: 'Only plan, never execute. Explore safely.',       'zh-CN': '仅规划,从不执行。安全探索。' },
};

export default permissionMode;
