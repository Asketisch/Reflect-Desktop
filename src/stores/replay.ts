import type { ReflectRolloutRecord } from '@/utils/types';
import type { Turn, TurnItem } from './agentStore';

export function turnsFromRollout(records: ReflectRolloutRecord[]): Turn[] {
  const turns: Turn[] = [];
  for (const record of records) {
    const r = record as Record<string, unknown>;
    const id = String(r.turn_id ?? r.id ?? `replay-${turns.length}`);
    let turn = turns.find((item) => item.id === id);
    if (!turn) {
      turn = { id, items: [], status: 'done' };
      turns.push(turn);
    }
    const type = String(r.type ?? '');
    const text = typeof r.text === 'string' ? r.text : typeof r.content === 'string' ? r.content : '';
    let item: TurnItem | null = null;
    if (type.includes('user') && text) item = { kind: 'user_text', text };
    else if ((type.includes('assistant') || type.includes('agent')) && text) item = { kind: 'assistant_text', text };
    else if (type.includes('thinking') && text) item = { kind: 'thinking', text };
    else if (type.includes('error') && text) item = { kind: 'error', text };
    if (item) turn.items.push(item);
  }
  return turns;
}
