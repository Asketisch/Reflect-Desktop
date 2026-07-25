import type { Turn, TurnItem, TurnStatus } from './types';

export const EVENT_ID_NONE = '';

export function appendItem(turns: Turn[], turnId: string, item: TurnItem): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  const index = turns.findIndex((turn) => turn.id === turnId);
  if (index === -1) {
    return [...turns, { id: turnId, items: [item], status: 'streaming' }];
  }
  return turns.map((turn, i) =>
    i === index ? { ...turn, items: [...turn.items, item] } : turn,
  );
}

export function markTurn(turns: Turn[], turnId: string, status: TurnStatus): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  return turns.map((turn) => (turn.id === turnId ? { ...turn, status } : turn));
}

export function upsertDelta(turns: Turn[], turnId: string, delta: string): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    for (let i = items.length - 1; i >= 0; i--) {
      const item = items[i];
      if (item.kind === 'assistant_text') {
        items[i] = { ...item, text: item.text + delta, streaming: true };
        return { ...turn, items };
      }
    }
    items.push({ kind: 'assistant_text', text: delta, streaming: true });
    return { ...turn, items };
  });
}

export function upsertThinking(turns: Turn[], turnId: string, delta: string): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    for (let i = items.length - 1; i >= 0; i--) {
      const item = items[i];
      if (item.kind === 'thinking') {
        items[i] = { ...item, text: item.text + delta };
        return { ...turn, items };
      }
    }
    items.push({ kind: 'thinking', text: delta });
    return { ...turn, items };
  });
}

export function finalizeAssistantText(
  turns: Turn[],
  turnId: string,
  text: string,
): Turn[] {
  return mapTurn(turns, turnId, (turn) => {
    const items = [...turn.items];
    for (let i = items.length - 1; i >= 0; i--) {
      if (items[i].kind === 'assistant_text') {
        items[i] = { kind: 'assistant_text', text, streaming: false };
        return { ...turn, items };
      }
    }
    items.push({ kind: 'assistant_text', text, streaming: false });
    return { ...turn, items };
  });
}

function mapTurn(turns: Turn[], turnId: string, map: (turn: Turn) => Turn): Turn[] {
  if (turnId === EVENT_ID_NONE) return turns;
  const index = turns.findIndex((turn) => turn.id === turnId);
  if (index === -1) {
    return [...turns, map({ id: turnId, items: [], status: 'streaming' })];
  }
  return turns.map((turn, i) => (i === index ? map(turn) : turn));
}

export function summarizeArgs(args: unknown): string {
  if (args == null) return '';
  try {
    const summary = typeof args === 'string' ? args : JSON.stringify(args);
    return summary.length > 200 ? `${summary.slice(0, 200)}…` : summary;
  } catch {
    return String(args);
  }
}

export function summarizeToolOutput(output: unknown): string {
  if (output == null) return '';
  if (typeof output === 'string') return output;
  try {
    return JSON.stringify(output, null, 2);
  } catch {
    return String(output);
  }
}
