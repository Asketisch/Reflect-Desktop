/**
 * M1.4 MessageList —— 简化版 reducer:按 AgentMessage / AgentMessageDelta 拼接;
 *   启用 react-markdown 渲染。 完整 7 种 row + 虚拟列表留到 M2.5。
 */
import { useEffect, useRef } from 'react';
import { Markdown } from '@/components/Markdown';
import { useAgent, type Turn } from '@/services/agent';

const NEAR_BOTTOM_PX = 64;

export function MessageList() {
  const { turns, session } = useAgent();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (dist < NEAR_BOTTOM_PX) {
      el.scrollTop = el.scrollHeight;
    }
  }, [turns]);

  return (
    <>
      <p style={{ color: '#666', marginTop: 0, fontSize: 12 }}>
        session: {session ? `${session.model} @ ${session.provider}` : '(waiting...)'}
      </p>
      <div ref={ref} style={{ overflowY: 'auto', padding: '0 8px' }}>
        {turns.length === 0 && (
          <p style={{ color: '#888' }}>说点什么开始对话…</p>
        )}
        {turns.map((t) => (
          <TurnRow key={t.id} turn={t} />
        ))}
      </div>
    </>
  );
}

function TurnRow({ turn }: { turn: Turn }) {
  return (
    <div style={{ marginBottom: 16 }}>
      <UserBubble text={turn.user} />
      <AssistantBubble text={turn.reply} streaming={!turn.done} />
    </div>
  );
}

function UserBubble({ text }: { text: string }) {
  return (
    <div
      style={{
        background: '#dbeafe',
        padding: '8px 12px',
        borderRadius: 8,
        marginBottom: 6,
        maxWidth: '85%',
        marginLeft: 'auto',
      }}
    >
      {text}
    </div>
  );
}

function AssistantBubble({ text, streaming }: { text: string; streaming: boolean }) {
  return (
    <div
      style={{
        background: '#f1f5f9',
        padding: '8px 12px',
        borderRadius: 8,
        maxWidth: '90%',
        color: '#0f172a',
      }}
    >
      {text.length > 0 ? <Markdown text={text} /> : <p style={{ color: '#888' }}>…</p>}
      {streaming && <span style={{ opacity: 0.5 }}> ▍</span>}
    </div>
  );
}
