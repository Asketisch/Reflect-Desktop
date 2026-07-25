/**
 * i18n namespace —— prompts.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const prompts: Record<string, StringEntry> = {
  'prompts.title':     { en: 'Prompts',                     'zh-CN': '提示词' },
  'prompts.subtitle':  { en: 'Reusable prompt templates.',  'zh-CN': '可复用的提示词模板。' },
  'prompts.empty':     { en: 'No prompts yet.',             'zh-CN': '暂无提示词。' },
  'prompts.new':       { en: 'New prompt',                  'zh-CN': '新建提示词' },
  'prompts.copy':      { en: 'Copy to clipboard',           'zh-CN': '复制到剪贴板' },
  'prompts.insert':    { en: 'Insert into composer',        'zh-CN': '插入到输入框' },
};

export default prompts;
