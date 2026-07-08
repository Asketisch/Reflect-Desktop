/**
 * Time 工具 —— 相对时间、格式化。
 *
 * CodexMonitor 同名: `src/utils/time.ts`
 */

/** "30 minutes ago" / "2 days ago" / "just now"。 */
export function relativeTime(iso: string, now: number = Date.now()): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return '';
  const deltaMs = now - t;
  const future = deltaMs < 0;
  const ms = Math.abs(deltaMs);

  const sec = Math.floor(ms / 1000);
  if (sec < 60) return future ? 'in a moment' : 'just now';
  const min = Math.floor(sec / 60);
  if (min < 60) return future ? `in ${min} min` : `${min} min ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return future ? `in ${hr} h` : `${hr} h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return future ? `in ${day} d` : `${day} d ago`;
  const week = Math.floor(day / 7);
  if (week < 4) return future ? `in ${week} w` : `${week} w ago`;
  const month = Math.floor(day / 30);
  if (month < 12) return future ? `in ${month} mo` : `${month} mo ago`;
  const year = Math.floor(day / 365);
  return future ? `in ${year} y` : `${year} y ago`;
}

/** "12:34:56" 形式（24h）。 */
export function clockTime(date: Date | number = new Date()): string {
  const d = typeof date === 'number' ? new Date(date) : date;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** millisecond → "1.2s" / "150ms"。 */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const min = Math.floor(ms / 60_000);
  const sec = Math.floor((ms % 60_000) / 1000);
  return `${min}m ${sec}s`;
}