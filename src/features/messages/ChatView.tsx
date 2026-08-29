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
import { reflect_replay_session, reflect_git_diff } from '@/utils/commands';
import { reflect_bind_session } from '@/utils/commands/sessions';
import { useUiPrefs } from '@/utils/uiPrefs';
import type { ReflectRolloutRecord } from '@/utils/types';
import { useI18n } from '@/utils/i18n';
import { useAutoSessionTitle } from '@/features/sessions/hooks/useAutoSessionTitle';
import { ChatHero, QuickPromptRow } from '@/features/home/ChatHero';
import { ContextBanner } from './ContextBanner';
import { MessageSquare, GitBranch } from 'lucide-react';
import { Button, Icon, Spinner } from '@/features/design-system';
import { DiffViewer } from '@/features/git/DiffViewer';
import s from './ChatView.module.css';

export function ChatView() {
  const { sessionId } = useParams({ strict: false }) as { sessionId?: string };
  // B：本会话首个 turn 完成后自动生成 AI 标题（后端幂等，失败静默）。
  useAutoSessionTitle(sessionId);
  const turns = useAgentStore((st) => st.turns);
  const loadedSessionId = useAgentStore((st) => st.loadedSessionId);
  const hydrateSession = useAgentStore((st) => st.hydrateSession ?? (() => {}));
  const clearSession = useAgentStore((st) => st.clearSession ?? (() => {}));
  const syncPermissionMode = useAgentStore((st) => st.syncPermissionMode);
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
    const myId = ++requestIdRef.current;
    setLoading(true);
    setError(null);
    // v1.x:先 bind(后端幂等,同 id 短路)再决定 hydrate。bind 保证后端
    // AgentThread 落到本 session id(recorder + LLM 上下文),即使已水合
    // 也必须先走一遍(早退前),Composer 发送才会续写本会话文件。
    // 返回值是该会话恢复出的权限模式 —— 模式绑定会话而非进程,切换
    // 会话后用返回值同步本地徽标/切换器,避免沿用上一会话的显示。
    void reflect_bind_session(sessionId)
      .then((mode) => {
        if (requestIdRef.current !== myId) return null;
        syncPermissionMode(mode);
        // 已水合同 id:bind 已完成,不再重复 replay。读 store 实时值而非
        // 闭包快照 —— `loadedSessionId` 不能进 deps(水合成功后会让本
        // effect 重跑第二遍:多余 bind + loading banner 闪断)。
        if (useAgentStore.getState().loadedSessionId === sessionId) return null;
        return reflect_replay_session(sessionId);
      })
      .then((records: ReflectRolloutRecord[] | null) => {
        if (requestIdRef.current !== myId) return;
        if (records) hydrateSession(sessionId, records);
      })
      .catch((e: unknown) => {
      if (requestIdRef.current !== myId) return;
      setError(e instanceof Error ? e.message : String(e));
    }).finally(() => {
      if (requestIdRef.current !== myId) return;
      setLoading(false);
    });
    return () => {
      // unmount / 换 session 时使在途请求作废,防止旧 replay 水合进
      // 全局 store 污染新会话视图。
      requestIdRef.current++;
    };
  }, [sessionId, hydrateSession, clearSession, syncPermissionMode]);

  const isLoaded = !sessionId || loadedSessionId === sessionId;

  // 首页工作台英雄态：全新对话（无 session、无历史、无加载错误）时，
  // 问候语 + 快捷模板 chips + 居中 Composer 取代常规消息流。
  // 首条消息发出后 turns > 0，同一 Composer 实例自然落回底部常规布局。
  const hero = !sessionId && !loading && !error && turns.length === 0;

  // B13：chat + diff 分屏视图。在 Settings → Display 中切换。
  const [prefs] = useUiPrefs();
  const [diffText, setDiffText] = useState<string | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  useEffect(() => {
    if (!prefs.chatDiffSplit) {
      setDiffText(null);
      return;
    }
    setDiffLoading(true);
    reflect_git_diff(false)
      .then((d) => setDiffText(d))
      .catch(() => setDiffText(''))
      .finally(() => setDiffLoading(false));
  }, [prefs.chatDiffSplit]);

  return (
    <div className={s.root} data-split={prefs.chatDiffSplit ? 'on' : 'off'} data-hero={hero ? 'on' : 'off'}>
      {sessionId && loading && (
        <div className={s.sessionBanner} role="status" aria-live="polite">
          <Spinner size={14} />
          <span>{t('chat.loading')}</span>
        </div>
      )}
      {sessionId && error && (
        <div className={s.sessionBanner} role="alert">
          <span>{t('chat.loadErrorWithMsg', { msg: error })}</span>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              const myId = ++requestIdRef.current;
              setError(null);
              setLoading(true);
              // 与主加载序列同构:bind(幂等)→ replay → hydrate。
              void reflect_bind_session(sessionId)
                .then((mode) => {
                  syncPermissionMode(mode);
                  return reflect_replay_session(sessionId);
                })
                .then((r) => {
                  if (requestIdRef.current !== myId) return;
                  hydrateSession(sessionId, r);
                }).catch((e: unknown) => {
                  if (requestIdRef.current !== myId) return;
                  setError(e instanceof Error ? e.message : String(e));
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
      {hero && <ChatHero />}
      {!hero && <ContextBanner />}
      {!loading && !error && (
        // 英雄态仅视觉隐藏（display:none），保持 MessageList 挂载 ——
        // 其 role="log" 区与滚动锚点被集成测试和自动滚动消费。
        <div className={s.chatArea} data-hero={hero ? 'on' : 'off'} data-testid="chat-messages-area">
          <MessageList />
        </div>
      )}
      {!loading && !error && <Composer />}
      {hero && <QuickPromptRow />}
      {prefs.chatDiffSplit && !hero && (
        <aside className={s.diffPanel} aria-label={t('chat.diffPanel')}>
          <header className={s.diffHeader}>
            <Icon icon={GitBranch} size={14} />
            <span>{t('chat.diffHeader')}</span>
          </header>
          <div className={s.diffPre}>
            <DiffViewer
              diff={diffLoading ? '' : (diffText ?? '')}
              emptyMessage={diffLoading ? t('chat.diffLoading') : t('chat.diffEmpty')}
            />
          </div>
        </aside>
      )}
    </div>
  );
}