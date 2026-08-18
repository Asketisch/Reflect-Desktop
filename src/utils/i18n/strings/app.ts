/**
 * i18n 命名空间 —— app.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const app: Record<string, StringEntry> = {
  'app.title':  { en: 'Reflect Desktop',  'zh-CN': 'Reflect Desktop' },
  'app.name':   { en: 'Reflect',          'zh-CN': 'Reflect' },
};

export default app;
