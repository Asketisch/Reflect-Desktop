/**
 * i18n namespace —— mobile.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const mobile: Record<string, StringEntry> = {
  'mobile.title':     { en: 'Mobile',                                   'zh-CN': '移动' },
  'mobile.subtitle':  { en: 'Companion mobile companion integration.',  'zh-CN': '移动端伴侣集成。' },
  'mobile.pair':      { en: 'Pair device',                              'zh-CN': '配对设备' },
  'mobile.unpair':    { en: 'Unpair',                                   'zh-CN': '取消配对' },
  'mobile.empty':     { en: 'No paired devices.',                       'zh-CN': '暂无配对设备。' },
};

export default mobile;
