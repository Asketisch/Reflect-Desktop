/**
 * ChatView —— MessageList + Composer + session replay 加载流程。
 *
 * 阶段 A2：读 route param sessionId，显示当前 session 上下文。
 * 阶段 B：调用 `reflect_replay_session`，把 history hydrate 到 agentStore，
 * 防止切到新 session 时仍展示上一条 chat 的 turns。
 */
import { useParams } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { MessageList } from './MessageList';
import { Composer } from './Composer';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_replay_session } from '@/utils/tauri';
import type { ReflectRolloutRecord } from '@/utils/types';
import { useI18n } from '@/utils/i18n';
import { MessageSquare } from 'lucide-react';
import { Button, Icon, Spinner } from '@/features/design-system';
import s from './ChatView.module.css';

export function ChatView() {
  const { sessionId } = useParams({ strict: false }) as { sessionId?: string };
  const turns = useAgentStore((st) => st.turns);
  const loadedSessionId = useAgentStore((st) => st.loadedSessionId);
  const hydrateSession = useAgentStore((st) => st.hydrateSession ?? (() => {}));
  const clearSession = useAgentStore((st) => st.clearSession ?? (() => {}));
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (!sessionId) {
      clearSession();
      setLoading(false);
      setError(null);
      return;
    }
    if (loadedSessionId === sessionId) {
      setLoading(false);
      setError(null);
      return;
    }
    const myId = ++requestIdRef.current;
    setLoading(true);
    setError(null);
    void reflect_replay_session(sessionId).then((records: ReflectRolloutRecord[]) => {
      if (requestIdRef.current !== myId) return;
      hydrateSession(sessionId, records);
    }).catch((e: unknown) => {
      if (requestIdRef.current !== myId) return;
      setError(e instanceof Error ? e.message : String(e));
    }).finally(() => {
      if (requestIdRef.current !== myId) return;
      setLoading(false);
    });
  }, [sessionId, loadedSessionId, hydrateSession, clearSession]);

  const isLoaded = !sessionId || loadedSessionId === sessionId;
  return (
    <div className={s.root}>
      {sessionId && loading && (
        <div className={s.sessionBanner} role="status" aria-live="polite">
          <Spinner size={14} />
          <span>{t('chat.loading')}</span>
        </div>
      )}
      {sessionId && error && (
        <div className={s.sessionBanner} role="alert">
          <span>{t('chat.loadError')} {error}</span>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              const myId = ++requestIdRef.current;
              setError(null);
              setLoading(true);
              void reflect_replay_session(sessionId).then((r) => {
                if (requestIdRef.current !== myId) return;
                hydrateSession(sessionId, r);
              }).catch((e) => {
                if (requestIdRef.current !== myId) return;
                setError(String(e));
              }).finally(() => {
                if (requestIdRef.current !== myId) return;
                setLoading(false);
              });
            }}
          >
            {t('chat.retry')}
          </Button>
        </div>
      )}
      {sessionId && isLoaded && !loading && !error && turns.length === 0 && (
        <div className={s.sessionBanner} role="status">{t('chat.emptySession')}</div>
      )}
      {sessionId && isLoaded && turns.length > 0 && (
        <div className={s.sessionBanner} role="status">
          <Icon icon={MessageSquare} size={12} />
          <span>{t('chat.viewing')} <code className={s.sessionId}>{sessionId}</code>.</span>
        </div>
      )}
      {!loading && !error && <><MessageList /><Composer /></>}
    </div>
  );
}