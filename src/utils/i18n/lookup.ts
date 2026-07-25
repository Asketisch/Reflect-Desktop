/**
 * 静态 lookup —— 不依赖 React context。
 *
 * 使用 STRINGS dict 提供 fallback（locale 缺失 → DEFAULT_LOCALE → 返 key）。
 */
import { DEFAULT_LOCALE } from './locale';
import { interpolate } from './interpolate';
import { STRINGS } from './strings';
import type { Locale } from './types';
import type { LocaleKey } from './strings';

/** 静态 lookup + interpolate. */
export function t(key: LocaleKey, locale: Locale = DEFAULT_LOCALE, vars?: Record<string, string | number>): string {
  const entry = STRINGS[key];
  const raw = entry?.[locale] ?? entry?.[DEFAULT_LOCALE] ?? key;
  return interpolate(raw, vars);
}

/** 把 string 中的 {count} 替换成 count+s(英文) / count(中文)。 */
export function pluralize(key: LocaleKey, locale: Locale, count: number, vars?: Record<string, string | number>): string {
  const all = { ...(vars ?? {}), count };
  if (locale === 'zh-CN') {
    // 中文模板里不应出现 {plural} —— 为安全起见,把所有 {plural} 替换成空串。
    return interpolate(t(key, locale, all), { plural: '', ...all });
  }
  // 英文：附加 's'。
  const plural = count === 1 ? '' : 's';
  return t(key, locale, { ...all, plural });
}