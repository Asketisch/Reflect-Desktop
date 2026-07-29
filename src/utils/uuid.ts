/**
 * UUID / id 工具 —— 生成 + 解析。
 */

/** UUID v4 fallback。Tauri WebView2 / 现代浏览器 / Node 18+ 都有 crypto.randomUUID。 */
export function uuid(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

/** 短 id（8 字符）。用于 session / thread 行显示。 */
export function shortId(id: string): string {
  return id.slice(0, 8);
}