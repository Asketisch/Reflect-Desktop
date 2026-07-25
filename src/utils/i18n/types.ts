/**
 * i18n 类型定义。
 *
 * Locale / LocaleKey / Strings 由各 domain namespace 模块的合并 catalog 推导，
 * 保留所有现有拼写错误的编译期报错行为。
 */
export type Locale = 'en' | 'zh-CN';

/** 单条翻译条目 —— 运行时允许部分 locale 缺失（fallback 兜底）。 */
export type StringEntry = { en: string; 'zh-CN': string };

/** 整个 catalog —— 由 strings/index.ts merge 所有 domain modules 得到。 */
export type Strings = Record<string, StringEntry>;