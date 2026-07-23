/**
 * src/services/agentEventBus.ts — B1-06 fan-out event bus.
 *
 * Architecture:
 *   - Single Tauri `reflect_event` subscription lives here (in this module).
 *   - Multiple consumers (agentStore, Inspector, Notifications, DevTools, …)
 *     call `subscribeAgentEvent(callback)` and each receives every event.
 *   - Fan-out is one-to-many: the underlying Tauri `listen` is shared so we
 *     don't open N independent webview listeners for the same channel.
 *   - Consumer callbacks are isolated — a throwing consumer does not affect
 *     other consumers.
 *
 * Usage:
 *   // In agentStore:
 *   useEffect(() => {
 *     const un = subscribeAgentEvent((e) => applyPatch(e));
 *     return un;
 *   }, []);
 *
 *   // In a dev/inspector panel:
 *   useEffect(() => {
 *     const un = subscribeAgentEvent((e) => recordEvent(e));
 *     return un;
 *   }, []);
 *
 * The first call to `subscribeAgentEvent` opens the Tauri listener (idempotent).
 * The last `unsubscribe` closes it. This is handled by an internal refcount.
 */
import { onReflectEvent } from '@/utils/commands';
import type { ReflectEvent } from '@/types/protocol';

type AgentEventHandler = (event: ReflectEvent) => void;

let __listeners: Set<AgentEventHandler> = new Set();
let __unlisten: (() => void) | null = null;
let __refCount = 0;
let __opening: Promise<void> | null = null;

/**
 * Subscribe to all `reflect_event` events. Returns an unsubscribe function.
 *
 * Lazy-connection: the underlying Tauri `listen` is opened on the first
 * subscribe and held open until every subscriber has unsubscribed.
 */
export function subscribeAgentEvent(handler: AgentEventHandler): () => void {
  if (__listeners.has(handler)) {
    // Idempotent: re-subscribing with the same fn returns the same cleanup.
    return () => unsubscribeAgentEvent(handler);
  }
  __listeners.add(handler);
  __refCount += 1;
  if (!__unlisten && !__opening) {
    __opening = openListener();
  }
  return () => unsubscribeAgentEvent(handler);
}

function unsubscribeAgentEvent(handler: AgentEventHandler): void {
  if (!__listeners.delete(handler)) return;
  __refCount -= 1;
  if (__refCount <= 0) {
    __refCount = 0;
    closeListener();
  }
}

async function openListener(): Promise<void> {
  try {
    const un = await onReflectEvent((e) => {
      // Fan-out: iterate over a snapshot to allow handlers to unsubscribe
      // themselves synchronously during dispatch without breaking iteration.
      for (const handler of Array.from(__listeners)) {
        try {
          handler(e);
        } catch (err) {
          // Isolated error: log but don't break other consumers.
          // eslint-disable-next-line no-console
          console.error('[agentEventBus] consumer threw', err);
        }
      }
    });
    // If everyone unsubscribed while we were awaiting, close immediately.
    if (__refCount <= 0) {
      un();
      return;
    }
    __unlisten = un;
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error('[agentEventBus] failed to open Tauri listener', e);
  } finally {
    __opening = null;
  }
}

function closeListener(): void {
  if (__unlisten) {
    try {
      __unlisten();
    } catch {
      // ignore
    }
    __unlisten = null;
  }
  __listeners.clear();
}

/** Test-only: reset module state. */
export function __resetAgentEventBusForTests(): void {
  closeListener();
  __refCount = 0;
  __listeners = new Set();
  __opening = null;
}

/** Diagnostics: how many subscribers are currently active. */
export function agentEventBusSubscriberCount(): number {
  return __refCount;
}
