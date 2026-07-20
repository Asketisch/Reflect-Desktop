/**
 * Sidebar —— 会话列表容器（CSS Modules 版）。
 *
 * 顶部：New Chat 主按钮 + 搜索框。
 * 主体：时间分桶 session 列表（BucketGroup + SessionItem）。
 *
 * 契约（Sidebar.test.tsx）：
 *   - 含 'Sessions' 标题文字
 *   - 空态显示 'No sessions yet'
 *   - loading 显示 'Loading'
 *   - error 直接渲染文案
 *   - refresh 按钮带 title="Refresh"
 */
import { useState, useMemo } from 'react';
import { Plus, RefreshCw, Search } from 'lucide-react';
import { Icon, IconButton, Button, Input } from '@/features/design-system';
import type { SessionBucket } from '../utils/buckets';
import { BucketGroup } from './BucketGroup';
import s from './Sidebar.module.css';

interface Props {
  buckets: SessionBucket[];
  loading: boolean;
  error: string | null;
  activeId: string | null;
  onSelect: (id: string) => void;
  onRefresh: () => void;
  onNewChat?: () => void;
}

export function Sidebar({ buckets, loading, error, activeId, onSelect, onRefresh, onNewChat }: Props) {
  const [query, setQuery] = useState('');

  const filtered = useMemo(() => {
    if (!query.trim()) return buckets;
    const q = query.toLowerCase();
    return buckets
      .map((b) => ({
        ...b,
        sessions: b.sessions.filter((s) =>
          (s.display_name || s.session_id).toLowerCase().includes(q),
        ),
      }))
      .filter((b) => b.sessions.length > 0);
  }, [buckets, query]);

  return (
    <div className={s.root}>
      <div className={s.header}>
        <h3 className={s.title}>Sessions</h3>
        <div className={s.headerActions}>
          <IconButton label="Refresh" onClick={onRefresh} disabled={loading} size="sm">
            <Icon icon={RefreshCw} size={14} />
          </IconButton>
        </div>
      </div>

      <div className={s.actions}>
        <Button variant="primary" block size="sm" onClick={onNewChat} leftIcon={<Icon icon={Plus} size={14} />}>
          New chat
        </Button>
        <Input
          size="sm"
          leading={<Icon icon={Search} size={13} />}
          placeholder="Search sessions…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Search sessions"
        />
      </div>

      <div className={s.list}>
        {error && <p className={s.error}>{error}</p>}
        {loading && filtered.length === 0 && <p className={s.hint}>Loading…</p>}
        {!loading && filtered.length === 0 && !error && (
          <p className={s.hint}>No sessions yet. Start a turn above to create one.</p>
        )}
        {filtered.map((b) => (
          <BucketGroup key={b.label} bucket={b} activeId={activeId} onSelect={onSelect} />
        ))}
      </div>
    </div>
  );
}
