/**
 * React Context —— 在 tree 顶层挂载 I18nProvider，组件用 useI18n() 拿 locale / t / tp。
 *
 * 启动时同步 <html lang>; 切换时写 localStorage。
 */
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';

import { DEFAULT_LOCALE, detectLocale, dictationLangFor, saveLocale } from './locale';
import { pluralize, t } from './lookup';
import type { Locale } from './types';
import type { LocaleKey } from './strings';

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** 静态 lookup,返回 string. */
  t: (key: LocaleKey, vars?: Record<string, string | number>) => string;
  /** plural helper. */
  tp: (key: LocaleKey, count: number, vars?: Record<string, string | number>) => string;
  /** 当前 locale 下的 dictation 语言代码. */
  dictationLang: string;
  /** 是否当前是中文. */
  isZh: boolean;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);

  // 同步 <html lang>.
  useEffect(() => {
    document.documentElement.setAttribute('lang', locale);
  }, [locale]);

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    saveLocale(next);
  }, []);

  const value = useMemo<I18nContextValue>(() => {
    return {
      locale,
      setLocale,
      t: (key, vars) => t(key, locale, vars),
      tp: (key, count, vars) => pluralize(key, locale, count, vars),
      dictationLang: dictationLangFor(locale),
      isZh: locale === 'zh-CN',
    };
  }, [locale, setLocale]);

  return React.createElement(I18nContext.Provider, { value }, children);
}

export function useI18n(): I18nContextValue {
  const value = useContext(I18nContext);
  if (!value) {
    // Fallback for tests / pre-mount contexts.
    return {
      locale: DEFAULT_LOCALE,
      setLocale: saveLocale,
      t: (key, vars) => t(key, DEFAULT_LOCALE, vars),
      tp: (key, count, vars) => pluralize(key, DEFAULT_LOCALE, count, vars),
      dictationLang: dictationLangFor(DEFAULT_LOCALE),
      isZh: false,
    };
  }
  return value;
}