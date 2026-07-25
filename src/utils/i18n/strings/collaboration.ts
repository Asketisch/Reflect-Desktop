/**
 * i18n namespace —— collaboration.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const collaboration: Record<string, StringEntry> = {
  'collaboration.title':     { en: 'Collaboration',                   'zh-CN': '协作' },
  'collaboration.subtitle':  { en: 'Share sessions with your team.',  'zh-CN': '与团队共享会话。' },
  'collaboration.empty':     { en: 'No collaborators yet.',           'zh-CN': '暂无协作者。' },
  'collaboration.invite':    { en: 'Invite',                          'zh-CN': '邀请' },
  'collaboration.share':     { en: 'Share',                           'zh-CN': '分享' },
};

export default collaboration;
