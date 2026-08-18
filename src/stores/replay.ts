import type { ReflectRolloutRecord } from '@/utils/types';
import type { Turn, TurnItem } from './agent/types';

/**
 * 从会话回放流构建转数列表。按 `seq` 顺序遍历记录，
 * 并按 `turn_id` 信封 id（`event.id`，见 AGENTS.md 协议不变量）
 * 依据 `payload.msg.type` 对事件分组。
 *
 * 记录为 `ReflectRolloutRecord { seq, kind, timestamp, payload }`，
 * 其中 `payload` 是 ReflectSubmission 或 ReflectEvent。
 * `event.id` 即为转数 id（会话级事件为 `EVENT_ID_NONE`）。
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
