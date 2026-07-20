/**
 * Home —— 欢迎仪表盘（CSS Modules 版）。
 *
 * - 顶部欢迎区 + 当前 model/workspace 状态卡
 * - 快速操作: 4 个大卡片（New Chat / Workspaces / Settings / Models）
 * - Recent Sessions 卡片列表
 */
import { useMemo } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { MessageSquarePlus, FolderOpen, Settings as SettingsIcon, Cpu, MessageSquare, ArrowRight } from 'lucide-react';
import type { ComponentType } from 'react';
import { useSessions } from '@/features/sessions/hooks/useSessions';
import { useAgent } from '@/services/agent';
import { Card, Badge, Icon, Button, EmptyState } from '@/features/design-system';
import { relativeTime } from '@/utils/time';
import s from './HomeView.module.css';

interface QuickAction {
  to: string;
  label: string;
  desc: string;
  icon: ComponentType;
}

const ACTIONS: QuickAction[] = [
  { to: '/chat', label: 'New chat', desc: 'Start a fresh conversation', icon: MessageSquarePlus },
  { to: '/workspaces', label: 'Workspaces', desc: 'Recent project directories', icon: FolderOpen },
  { to: '/models', label: 'Models', desc: 'Switch model or effort', icon: Cpu },
  { to: '/settings', label: 'Settings', desc: 'Provider & permissions', icon: SettingsIcon },
];

export function HomeView() {
  const { buckets } = useSessions();
  const { session } = useAgent();
  const navigate = useNavigate();

  const recent = useMemo(() => {
    const all: Array<{ id: string; label: string; started_at: string; tokens: number }> = [];
    for (const b of buckets) {
      for (const sess of b.sessions) {
        all.push({
          id: sess.session_id,
          label: sess.display_name || sess.session_id,
          started_at: sess.started_at,
          tokens: sess.token_total,
        });
      }
    }
    all.sort((a, b) => b.started_at.localeCompare(a.started_at));
    return all.slice(0, 6);
  }, [buckets]);

  return (
    <div className={s.root}>
      <header className={s.hero}>
        <div className={s.brand}>
          <div className={s.logo}>R</div>
          <div>
            <h1 className={s.title}>Welcome to Reflect</h1>
            <p className={s.subtitle}>
              AI coding agent — start a conversation or pick up where you left off.
            </p>
          </div>
        </div>
        {session && (
          <Card level="outlined" padding="sm" className={s.statusCard}>
            <div className={s.statusRow}>
              <Badge variant="success" dot>active</Badge>
              <code className={s.statusModel}>{session.model}</code>
              <span className={s.statusProvider}>@ {session.provider}</span>
            </div>
          </Card>
        )}
      </header>

      <section>
        <h2 className={s.sectionTitle}>Quick start</h2>
        <div className={s.actionGrid}>
          {ACTIONS.map((a) => (
            <button
              key={a.to}
              className={s.actionCard}
              onClick={() => navigate({ to: a.to })}
            >
              <div className={s.actionIcon}>
                <Icon icon={a.icon} size={20} />
              </div>
              <div className={s.actionBody}>
                <div className={s.actionLabel}>{a.label}</div>
                <div className={s.actionDesc}>{a.desc}</div>
              </div>
              <Icon icon={ArrowRight} size={14} className={s.actionArrow} />
            </button>
          ))}
        </div>
      </section>

      <section>
        <div className={s.sectionHeader}>
          <h2 className={s.sectionTitle}>Recent sessions</h2>
          <Button variant="ghost" size="sm" onClick={() => navigate({ to: '/sessions' })}>
            View all
          </Button>
        </div>
        {recent.length === 0 ? (
          <Card level="flat" padding="none">
            <EmptyState
              icon={<Icon icon={MessageSquare} />}
              title="No sessions yet"
              description="Start a chat to create your first session."
              action={
                <Button variant="primary" size="sm" leftIcon={<Icon icon={MessageSquarePlus} size={14} />} onClick={() => navigate({ to: '/chat' })}>
                  New chat
                </Button>
              }
            />
          </Card>
        ) : (
          <div className={s.recentList}>
            {recent.map((sess) => (
              <button
                key={sess.id}
                className={s.recentItem}
                onClick={() => navigate({ to: '/chat/$sessionId', params: { sessionId: sess.id } })}
              >
                <Icon icon={MessageSquare} size={14} className={s.recentIcon} />
                <div className={s.recentBody}>
                  <div className={s.recentLabel}>{sess.label}</div>
                  <div className={s.recentMeta}>
                    {relativeTime(sess.started_at)} · {sess.tokens} tok
                  </div>
                </div>
                <Icon icon={ArrowRight} size={12} className={s.actionArrow} />
              </button>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
