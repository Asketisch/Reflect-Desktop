/**
 * i18n namespace —— apps.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const apps: Record<string, StringEntry> = {
  'apps.title':     { en: 'Apps',                               'zh-CN': '应用' },
  'apps.subtitle':  { en: 'Loaded apps and MCP integrations.',  'zh-CN': '已加载的应用与 MCP 集成。' },
  'apps.empty':     { en: 'No apps loaded.',                    'zh-CN': '暂无应用。' },
};

export default apps;
