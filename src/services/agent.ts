/**
 * M1.4 useAgent —— 协议桥 hook + reducer。
 *
 * - 输入文本 → invoke `reflect_submit`
 * - listen `reflect_event` → 累积 AgentMessageDelta 流式输出
 * - 收到 AgentMessage 或 TurnComplete → 标记 turn 结束
 *
 * M2.5 升级到持久历史 LRU + 多 session 切换。
 */
import { useEffect, useState, useCallback } from 'react';
import { reflect_submit, onReflectEvent } from '@/utils/tauri';
import type { ReflectEvent, ReflectSubmission } from '@/types/protocol';

export interface Turn {
  id: string;       // submission.id
  user: string;
  reply: string;    // 流式累积
  done: boolean;
}

export interface AgentSession {
  model: string;
  provider: string;
}

const uuid = () =>
  (typeof crypto !== 'undefined' && 'randomUUID' in crypto)
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

/**
 * Hook: 订阅 reflect_event + 提供 submit。
 */
export function useAgent() {
  const [turns, setTurns] = useState<Turn[]>([]);
  const [session, setSession] = useState<AgentSession | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    (async () => {
      const fn = await onReflectEvent((e: ReflectEvent) => {
        if (cancelled) return;
        handle_event(setTurns, setSession, e);
      });
      unlisten = fn;
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const submit = useCallback(async (text: string) => {
    const id = uuid();
    const submission: ReflectSubmission = {
      id,
      op: { type: 'user_input', items: [{ type: 'text', text }] },
    };
    setTurns((prev) => [...prev, { id, user: text, reply: '', done: false }]);
    await reflect_submit(submission);
  }, []);

  return { turns, session, submit };
}

function handle_event(
  setTurns: React.Dispatch<React.SetStateAction<Turn[]>>,
  setSession: React.Dispatch<React.SetStateAction<AgentSession | null>>,
  e: ReflectEvent,
): void {
  const { msg } = e;
  switch (msg.type) {
    case 'session_configured': {
      const m = String(msg.model ?? '');
      const p = String(msg.provider ?? '');
      if (m) setSession({ model: m, provider: p });
      return;
    }
    case 'agent_message_delta': {
      const delta = String(msg.delta ?? '');
      setTurns((prev) =>
        prev.map((t) => (t.id === e.id ? { ...t, reply: t.reply + delta } : t)),
      );
      return;
    }
    case 'agent_message': {
      const text = String(msg.text ?? '');
      setTurns((prev) =>
        prev.map((t) => (t.id === e.id ? { ...t, reply: text, done: true } : t)),
      );
      return;
    }
    case 'turn_complete': {
      setTurns((prev) =>
        prev.map((t) => (t.id === e.id ? { ...t, done: true } : t)),
      );
      return;
    }
    default:
      return;
  }
}
