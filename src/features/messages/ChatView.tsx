/**
 * ChatView —— MessageList + Composer。
 *
 * 阶段 A2：读 route param sessionId，显示当前 session 上下文。
 * 真实 session replay（清 turns + 应用 rollout）留作后续；
 * 当前显示 banner 提示用户正在查看哪个 session。
 */
import { useParams } from '@tanstack/react-router';
import { MessageList } from './MessageList';
import { Composer } from './Composer';
import { useAgentStore } from '@/stores/agentStore';
import { MessageSquare } from 'lucide-react';
import { Icon } from '@/features/design-system';
import s from './ChatView.module.css';

export function ChatView() {
  const { sessionId } = useParams({ strict: false }) as { sessionId?: string };
  const turnsCount = useAgentStore((st) => st.turns.length);

  return (
    <div className={s.root}>
      {sessionId && turnsCount > 0 && (
        <div className={s.sessionBanner} role="status">
          <Icon icon={MessageSquare} size={12} />
          <span>
            Viewing session <code className={s.sessionId}>{sessionId}</code>. New messages start a fresh turn.
          </span>
        </div>
      )}
      <MessageList />
      <Composer />
    </div>
  );
}
