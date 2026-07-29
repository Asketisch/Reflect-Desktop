import type { ReflectRolloutRecord } from '@/utils/types';
import type { Turn, TurnItem } from './agent/types';

/**
 * Build turn list from a session rollout stream. Walks the records in
 * `seq` order and groups events by `payload.msg.type` against the
 * `turn_id` envelope id (`event.id` per AGENTS.md protocol invariants).
 *
 * Records are `ReflectRolloutRecord { seq, kind, timestamp, payload }`
 * where `payload` is either a ReflectSubmission or ReflectEvent. The
 * `event.id` is the turn id (or `EVENT_ID_NONE` for session-level events).
 */
export function turnsFromRollout(records: ReflectRolloutRecord[]): Turn[] {
  const turns: Turn[] = [];
  for (const record of records) {
    if (record.kind !== 'event') continue;
    const event = record.payload as { id: string; msg: { type: string; turn_id?: string; text?: string } };
    const id = event.id;
    let turn = turns.find((item) => item.id === id);
    if (!turn) {
      turn = { id, items: [], status: 'done' };
      turns.push(turn);
    }
    const type = event.msg.type;
    const text = event.msg.text ?? '';
    let item: TurnItem | null = null;
    if (type === 'agent_message' && text) item = { kind: 'assistant_text', text };
    else if (type === 'thinking_delta' && text) item = { kind: 'thinking', text };
    else if (type === 'turn_aborted' || type === 'error' || type === 'stream_error') {
      if (text) item = { kind: 'error', text };
    }
    if (item) turn.items.push(item);
  }
  return turns;
}
