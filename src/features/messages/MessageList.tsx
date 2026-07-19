/**
 * MessageList —— 富文本 chat 渲染。
 *
 * 阶段 3b:从旧 `{user, reply, done}` 字符串升级为按 `turn.items[]` 渲染多行:
 *   - user_text       用户气泡
 *   - assistant_text  助手 markdown 渲染(支持 streaming 指示)
 *   - thinking        可折叠灰文(reasoning)
 *   - tool_call       工具调用(可折叠:tool 名 + 参数 + 状态徽标)
 *   - tool_output     工具输出(配对 callId,可折叠;错误红框)
 *   - error           红色错误条
 *   - compacted       上下文压缩提示
 *
 * 虚拟化(@tanstack/react-virtual)留作后续优化;短会话直接渲染。
 */
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Markdown } from '@/components/Markdown';
import { useAgentStore, type Turn, type TurnItem } from '@/stores/agentStore';

const NEAR_BOTTOM_PX = 64;

export function MessageList() {
  const turns = useAgentStore((s) => s.turns);
  const session = useAgentStore((s) => s.session);
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
        {turns.length === 0 && <p style={{ color: '#888' }}>说点什么开始对话…</p>}
        {turns.map((t) => (
          <TurnView key={t.id} turn={t} />
        ))}
      </div>
    </>
  );
}

/** 单个 turn:渲染其 items 序列。 */
function TurnView({ turn }: { turn: Turn }) {
  return (
    <div style={{ marginBottom: 16 }}>
      {turn.items.map((item, i) => (
        <ItemView key={i} item={item} turnStatus={turn.status} />
      ))}
    </div>
  );
}

/** 单个 item 的渲染分发。 */
function ItemView({ item, turnStatus }: { item: TurnItem; turnStatus: Turn['status'] }) {
  switch (item.kind) {
    case 'user_text':
      return <UserBubble text={item.text} />;

    case 'assistant_text': {
      const streaming = item.streaming === true && turnStatus === 'streaming';
      return <AssistantBubble text={item.text} streaming={streaming} />;
    }

    case 'thinking':
      return <ThinkingBlock text={item.text} />;

    case 'tool_call':
      return <ToolCallRow item={item} />;

    case 'tool_output':
      return <ToolOutputRow text={item.text} isError={item.isError} />;

    case 'error':
      return <ErrorRow text={item.text} />;

    case 'compacted':
      return (
        <div
          style={{
            background: '#fef3c7',
            border: '1px solid #f59e0b',
            borderRadius: 6,
            padding: '6px 10px',
            margin: '8px 0',
            fontSize: 12,
            color: '#92400e',
          }}
        >
          ⚡ {item.summary}
        </div>
      );

    default:
      return null;
  }
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

/** thinking block —— 可折叠,默认折叠(减少噪音)。 */
function ThinkingBlock({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  return (
    <Collapsible
      label={`💭 thinking${text.length > 40 ? `: ${text.slice(0, 40)}…` : ''}`}
      open={open}
      onToggle={() => setOpen((o) => !o)}
    >
      <pre style={monoGrey}>{text}</pre>
    </Collapsible>
  );
}

/** 工具调用行 —— 可折叠,展示 tool 名 + 参数摘要 + 状态徽标。 */
function ToolCallRow({ item }: { item: Extract<TurnItem, { kind: 'tool_call' }> }) {
  const [open, setOpen] = useState(false);
  const statusBadge = item.status === 'running' ? '⏳' : item.status === 'done' ? '✓' : '✗';
  const label = `${statusBadge} ${item.toolName}(${item.argsSummary || ''})`;
  return (
    <Collapsible label={label} open={open} onToggle={() => setOpen((o) => !o)}>
      <pre style={{ ...monoGrey, color: '#475569' }}>{item.argsSummary || '(no args)'}</pre>
    </Collapsible>
  );
}

/** 工具输出行 —— 等宽,错误用红框。 */
function ToolOutputRow({ text, isError }: { text: string; isError: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <Collapsible
      label={isError ? '⨯ output (error)' : '↳ output'}
      open={open}
      onToggle={() => setOpen((o) => !o)}
      accent={isError ? '#ef4444' : undefined}
    >
      <pre
        style={{
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          margin: 0,
          fontSize: 12,
          color: isError ? '#991b1b' : '#334155',
          maxHeight: 320,
          overflow: 'auto',
        }}
      >
        {text}
      </pre>
    </Collapsible>
  );
}

function ErrorRow({ text }: { text: string }) {
  return (
    <div
      style={{
        background: '#fee2e2',
        border: '1px solid #ef4444',
        borderRadius: 6,
        padding: '8px 10px',
        margin: '6px 0',
        color: '#991b1b',
        fontSize: 13,
      }}
    >
      ⚠ {text}
    </div>
  );
}

const monoGrey: React.CSSProperties = {
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  margin: 0,
  color: '#6b7280',
  fontSize: 12,
  fontStyle: 'italic',
};

/** 可折叠容器(点击 header 切换)。 */
function Collapsible({
  label,
  open,
  onToggle,
  children,
  accent,
}: {
  label: ReactNode;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
  accent?: string;
}) {
  return (
    <div
      style={{
        margin: '4px 0',
        border: `1px solid ${accent ?? '#e2e8f0'}`,
        borderRadius: 6,
        overflow: 'hidden',
      }}
    >
      <button
        onClick={onToggle}
        style={{
          width: '100%',
          textAlign: 'left',
          background: accent ? `${accent}1a` : '#f8fafc',
          border: 'none',
          padding: '4px 8px',
          cursor: 'pointer',
          fontSize: 12,
          color: accent ?? '#475569',
          fontFamily: 'ui-monospace, monospace',
        }}
      >
        {open ? '▼' : '▶'} {label}
      </button>
      {open && <div style={{ padding: '6px 8px', background: '#fff' }}>{children}</div>}
    </div>
  );
}
