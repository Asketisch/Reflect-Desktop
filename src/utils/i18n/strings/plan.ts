/**
 * i18n namespace —— plan.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const plan: Record<string, StringEntry> = {
  'plan.title':     { en: 'Plan Mode',                                 'zh-CN': '计划模式' },
  'plan.subtitle':  { en: 'Draft a plan before executing.',            'zh-CN': '执行前先制定计划。' },
  'plan.enter':     { en: 'Enter plan mode',                           'zh-CN': '进入计划模式' },
  'plan.exit':      { en: 'Exit plan mode',                            'zh-CN': '退出计划模式' },
  'plan.empty':     { en: 'No plan yet — start a turn to draft one.',  'zh-CN': '暂无计划——开始对话来起草。' },
  'plan.approve':   { en: 'Approve plan',                              'zh-CN': '批准计划' },
  'plan.reject':    { en: 'Reject plan',                               'zh-CN': '拒绝计划' },
};

export default plan;
