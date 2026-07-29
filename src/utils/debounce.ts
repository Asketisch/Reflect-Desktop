/**
 * Debounce / throttle 工具 —— 用于输入防抖、滚动节流等。
 */

export function debounce<TArgs extends unknown[]>(
  fn: (...args: TArgs) => void,
  ms: number,
): (...args: TArgs) => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (...args: TArgs) => {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn(...args);
    }, ms);
  };
}

export function throttle<TArgs extends unknown[]>(
  fn: (...args: TArgs) => void,
  ms: number,
): (...args: TArgs) => void {
  let last = 0;
  let pending: ReturnType<typeof setTimeout> | null = null;
  return (...args: TArgs) => {
    const now = Date.now();
    const elapsed = now - last;
    if (elapsed >= ms) {
      last = now;
      fn(...args);
    } else {
      if (pending !== null) clearTimeout(pending);
      pending = setTimeout(() => {
        last = Date.now();
        pending = null;
        fn(...args);
      }, ms - elapsed);
    }
  };
}