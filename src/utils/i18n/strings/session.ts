import type { StringEntry } from '../types';

/**
 * session 归属相关文案。
 *
 * 命名分层(有意保留,不统一):数据模型 / API 用 `workspace`,
 * 用户文案用「未归属」—— 与 `workspaces.*` 的 i18n 惯例一致。
 */
const session: Record<string, StringEntry> = {
  'session.noWorkspace':    { en: 'Unassigned', 'zh-CN': '未归属' },
};

export default session;
