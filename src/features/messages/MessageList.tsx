/**
 * MessageList —— 富文本 chat 渲染（CSS Modules 版）。
 *
 * turn.items[] 渲染分发：
 *   - user_text       右对齐用户气泡
 *   - assistant_text  左对齐助手 markdown + streaming 光标
 *   - thinking        可折叠 reasoning
 *   - tool_call       可折叠 tool 调用（lucide 图标 + 状态徽标）
 *   - tool_output     可折叠输出（错误用 danger 主题）
 *   - error           红色错误条
 *   - compacted       上下文压缩提示
 *
 * session 状态行已移到 TitleBar（避免重复）。
 */
import { useEffect, useRef } from 'react';
import {
  Brain,
  Wrench,
  Zap,
  AlertTriangle,
  MessageSquare,
} from 'lucide-react';
import { Icon } from '@/features/design-system';
import { Markdown } from '@/components/Markdown';
import { useAgentStore, type Turn, type TurnItem } from '@/stores/agentStore';
import { Collapsible } from './Collapsible';
import { ToolCell } from './ToolCells';
import s from './MessageList.module.css';

const NEAR_BOTTOM_PX = 64;

export function MessageList() {
  const turns = useAgentStore((st) => st.turns);
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
    <div className={s.scroll} ref={ref} role="log" aria-label="Conversation" aria-live="polite" aria-atomic="false">
      <div className={s.inner}>
        {turns.length === 0 && (
          <div className={s.emptyWrap}>
            <Icon icon={MessageSquare} size={32} className={s.emptyIcon} />
            <p className={s.emptyTitle}>Start a conversation</p>
            <p className={s.emptyHint}>
              Type below or use a slash command. Press <kbd>⌘</kbd>/<kbd>Ctrl</kbd> + <kbd>Enter</kbd> to send.
            </p>
          </div>
        )}
        {turns.map((t) => (
          <TurnView key={t.id} turn={t} />
        ))}
      </div>
    </div>
  );
}

function TurnView({ turn }: { turn: Turn }) {
  return (
    <div className={s.turn}>
      {turn.items.map((item, i) => (
        <ItemView key={i} item={item} turnStatus={turn.status} />
      ))}
    </div>
  );
}

function ItemView({ item, turnStatus }: { item: TurnItem; turnStatus: Turn['status'] }) {
  switch (item.kind) {
    case 'user_text':
      return <UserBubble text={item.text} />;

    case 'assistant_text': {
      const streaming = item.streaming === true && turnStatus === 'streaming';
      return <AssistantBubble text={item.text} streaming={streaming} />;
    }

    case 'thinking':
      return (
        <Collapsible
          icon={<Icon icon={Brain} size={13} />}
          accent="info"
          label={item.text.length > 60 ? `thinking: ${item.text.slice(0, 60)}…` : 'thinking'}
        >
          <pre className={s.monoText}>{item.text}</pre>
        </Collapsible>
      );

    case 'tool_call': {
      // B7-05: per-tool rendering (icon, arg summary, collapsible raw args).
      return (
        <ToolCell
          toolName={item.toolName}
          argsSummary={item.argsSummary}
          status={item.status}
        />
      );
    }

    case 'tool_output':
      return (
        <Collapsible
          icon={<Icon icon={Wrench} size={13} />}
          accent={item.isError ? 'danger' : 'default'}
          label={item.isError ? 'output (error)' : 'output'}
        >
          <pre className={`${s.monoText} ${item.isError ? s.monoTextError : ''}`}>{item.text}</pre>
        </Collapsible>
      );

    case 'error':
      return (
        <div className={s.errorRow}>
          <Icon icon={AlertTriangle} size={14} />
          <span>{item.text}</span>
        </div>
      );

    case 'compacted':
      return (
        <div className={s.compactedRow}>
          <Icon icon={Zap} size={13} />
          <span>{item.summary}</span>
        </div>
      );

    default:
      return null;
  }
}

function UserBubble({ text }: { text: string }) {
  return (
    <div className={s.userRow}>
      <div className={s.userBubble}>{text}</div>
    </div>
  );
}

function AssistantBubble({ text, streaming }: { text: string; streaming: boolean }) {
  return (
    <div className={s.assistantRow}>
      <div className={s.assistantAvatar} aria-hidden="true">R</div>
      <div className={s.assistantBubble}>
        {text.length > 0 ? (
          <Markdown text={text} />
        ) : (
          <span className={s.thinking}>…</span>
        )}
        {streaming && <span className={s.cursor} aria-hidden="true" />}
      </div>
    </div>
  );
}
