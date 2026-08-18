/**
 * 实时 agent 输出的事件通道订阅。
 *
 * 对应 `src-tauri/src/state.rs` 的 `reflect_event` 广播 —— payload 形态
 * 定义在 `src/types/protocol.ts`。
 */
import { listen } from '../bridge';
import type { ReflectEvent } from '@/types/protocol';

/**
 * 订阅全局 `reflect_event` 通道。
 *
 * 返回的函数用于取消订阅(Tauri `UnlistenFn`)。
 */
export async function onReflectEvent(handler: (event: ReflectEvent) => void) {
  return listen<ReflectEvent>('reflect_event', (e) => handler(e.payload));
}
