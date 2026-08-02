/**
 * i18n namespace —— update.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const update: Record<string, StringEntry> = {
  'update.title':      { en: 'Updates',                        'zh-CN': '更新' },
  'update.subtitle':   { en: 'Reflect Desktop version info.',  'zh-CN': 'Reflect Desktop 版本信息。' },
  'update.checking':   { en: 'Checking for updates…',          'zh-CN': '正在检查更新…' },
  'update.available':  { en: 'Update available: {version}',    'zh-CN': '有新版本: {version}' },
  'update.upToDate':   { en: 'You are up to date.',            'zh-CN': '已是最新版本。' },
  'update.error':      { en: 'Could not check for updates.',   'zh-CN': '无法检查更新。' },
  'update.install':    { en: 'Install',                        'zh-CN': '安装' },
  'update.later':      { en: 'Later',                          'zh-CN': '稍后' },
  'update.currentVersion': { en: 'Current version',           'zh-CN': '当前版本' },
  'update.manual':     { en: 'manual',                        'zh-CN': '手动' },
  'update.howTo':      { en: 'How to update',                 'zh-CN': '如何更新' },
  'update.howText':    { en: 'Automatic updates are not integrated yet. Update manually:', 'zh-CN': '自动更新暂未集成。请手动更新：' },
};

export default update;
