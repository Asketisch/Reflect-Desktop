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
 *
 * 上下文压缩（context_compacted）不进对话流 —— 聚合统计见 Inspector 概览。
 * session 状态行已移到 TitleBar（避免重复）。
 */
import { useEffect, useRef } from 'react';
import {
  Brain,
  Wrench,
  AlertTriangle,
  MessageSquare,
  FileDiff,
} from 'lucide-react';
import { Icon } from '@/features/design-system';
import { Markdown } from '@/components/Markdown';
import { useAgentStore, type Turn, type TurnItem } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import { DiffViewer } from '@/features/git/DiffViewer';
import { Collapsible } from './Collapsible';
import { ToolCell } from './ToolCells';
import { QueuedMessages } from './QueuedMessages';
import { splitAssistantText } from './assistantText';
import s from './MessageList.module.css';

const NEAR_BOTTOM_PX = 64;

export function MessageList() {
  const turns = useAgentStore((st) => st.turns);
  const ref = useRef<HTMLDivElement>(null);
  const { t } = useI18n();

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (dist < NEAR_BOTTOM_PX) {
      el.scrollTop = el.scrollHeight;
    }
  }, [turns]);

  return (
    <div className={s.scroll} ref={ref} role="log" aria-label={t('chat.ariaLabel')} aria-live="polite" aria-atomic="false">
      <div className={s.inner}>
        {turns.length === 0 && (
          <div className={s.emptyWrap}>
            <Icon icon={MessageSquare} size={32} className={s.emptyIcon} />
            <p className={s.emptyTitle}>{t('chat.empty.title')}</p>
            <p className={s.emptyHint}>{t('chat.empty.hint')}</p>
          </div>
        )}
        {turns.map((t) => (
          <TurnView key={t.id} turn={t} />
        ))}
        <QueuedMessages />
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
  const { t } = useI18n();
  switch (item.kind) {
    case 'user_text':
      return <UserBubble text={item.text} />;

    case 'assistant_text': {
      const streaming = item.streaming === true && turnStatus === 'streaming';
      return <AssistantBubble text={item.text} streaming={streaming} />;
    }

    case 'thinking':
      return <ThinkingCollapsible text={item.text} />;

    case 'tool_call': {
      // B7-05：按工具渲染（图标、参数摘要、可折叠原始参数）。
      return (
        <ToolCell
          toolName={item.toolName}
          argsSummary={item.argsSummary}
          status={item.status}
        />
      );
    }

    case 'tool_output': {
      // edit/write 类输出带 unified diff → 用 DiffViewer 渲染改动，
      // 标题为目标文件路径；纯文本输出保持可折叠 pre。
      if (item.diff) {
        return (
          <Collapsible
            icon={<Icon icon={FileDiff} size={13} />}
            accent="default"
            label={item.path || t('chat.diffLabel')}
            defaultOpen
          >
            <DiffViewer diff={item.diff} />
            {item.text && <pre className={s.monoText}>{item.text}</pre>}
          </Collapsible>
        );
      }
      return (
        <Collapsible
          icon={<Icon icon={Wrench} size={13} />}
          accent={item.isError ? 'danger' : 'default'}
          label={item.isError ? t('chat.outputError') : t('chat.output')}
        >
          <pre className={`${s.monoText} ${item.isError ? s.monoTextError : ''}`}>{item.text}</pre>
        </Collapsible>
      );
    }

    case 'error':
      return (
        <div className={s.errorRow}>
          <Icon icon={AlertTriangle} size={14} />
          <span>{item.text}</span>
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

/** 思考过程折叠块（ThinkingDelta 事件与正文 <think> 分段共用）。
 *  文字用弱化色 —— 思考与正式输出视觉分级。 */
function ThinkingCollapsible({ text, defaultOpen = false }: { text: string; defaultOpen?: boolean }) {
  const { t } = useI18n();
  const snippet = text.slice(0, 60).replace(/\s+/g, ' ');
  return (
    <Collapsible
      icon={<Icon icon={Brain} size={13} />}
      accent="info"
      defaultOpen={defaultOpen}
      label={text.length > 60 ? t('chat.thinkingWith', { snippet }) : t('chat.thinking')}
    >
      <pre className={`${s.monoText} ${s.thinkingText}`}>{text}</pre>
    </Collapsible>
  );
}

function AssistantBubble({ text, streaming }: { text: string; streaming: boolean }) {
  // 模型把推理以 <think> 标签混进正文、runtime 的 nudge 提醒会让模型
  // 回显 FINAL ANSWER 标记 —— 都不属于正文,渲染前分段/剥离。
  const { thinking, content } = splitAssistantText(text);
  return (
    <div className={s.assistantRow}>
      <div className={s.assistantAvatar} aria-hidden="true">R</div>
      <div className={s.assistantBubble}>
        {thinking && (
          <ThinkingCollapsible
            // 正文出现后重挂载让折叠块自动收起(默认展开仅在纯思考阶段)。
            key={content ? 'with-content' : 'thinking-only'}
            text={thinking}
            defaultOpen={streaming && !content}
          />
        )}
        {content.length > 0 ? (
          <Markdown text={content} />
        ) : (
          !thinking && <span className={s.thinking}>…</span>
        )}
        {streaming && <span className={s.cursor} aria-hidden="true" />}
      </div>
    </div>
  );
}
