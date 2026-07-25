/**
 * Event channel subscription for live agent output.
 *
 * Mirrors `src-tauri/src/state.rs` `reflect_event` broadcast — payload shape
 * lives in `src/types/protocol.ts`.
 */
import { listen } from '../bridge';
import type { ReflectEvent } from '@/types/protocol';

/**
 * Subscribe to the global `reflect_event` channel.
 *
 * The returned function unlistens (Tauri `UnlistenFn`).
 */
export async function onReflectEvent(handler: (event: ReflectEvent) => void) {
  return listen<ReflectEvent>('reflect_event', (e) => handler(e.payload));
}
