/**
 * i18n namespace —— design.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const design: Record<string, StringEntry> = {
  'design.title':               { en: 'Design System',                                'zh-CN': '设计系统' },
  'design.subtitle':            { en: 'Tokens, primitives, and components catalog.',  'zh-CN': 'Tokens、primitives 与组件 catalog。' },
  'design.section.tokens':      { en: 'Tokens',                                       'zh-CN': 'Tokens' },
  'design.section.primitives':  { en: 'Primitives',                                   'zh-CN': 'Primitives' },
  'design.section.colors':      { en: 'Colors',                                       'zh-CN': '颜色' },
  'design.section.typography':  { en: 'Typography',                                   'zh-CN': '字体' },
  'design.section.spacing':     { en: 'Spacing',                                      'zh-CN': '间距' },
  'design.section.radii':       { en: 'Radii',                                        'zh-CN': '圆角' },
  'design.section.shadows':     { en: 'Shadows',                                      'zh-CN': '阴影' },
};

export default design;
