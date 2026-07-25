/**
 * 简单占位符插值 —— 仅支持 {key}。
 *
 * 缺值保留原 `{key}` 占位符，避免掩盖 typo。
 */
export function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) => {
    const v = vars[key];
    return v === undefined || v === null ? `{${key}}` : String(v);
  });
}