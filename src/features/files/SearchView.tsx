/**
 * SearchView —— Settings > Files or dedicated page: Find-in-Files。
 *
 * 通过后端 `reflect_search_files` 搜索工作区文本,跳过 .git / node_modules 等。
 *
 * 输入即查(search-as-you-type),200 hit 上限(可调),
 * 点击 hit 在目标文件打开(跳到 Files 视图 + 选中文件)。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Search, X } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Spinner, EmptyState } from '@/features/design-system';
import {
  reflect_search_files,
  type ReflectFileSearchHit,
} from '@/utils/commands';
import s from './SearchView.module.css';

export function SearchView() {
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<ReflectFileSearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [truncated, setTruncated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<number | null>(null);
  // Monotonic request token: out-of-order responses are dropped so a slow
  // older query can never overwrite a fresher one.
  const seqRef = useRef(0);

  const runSearch = useCallback(async (q: string) => {
    const trimmed = q.trim();
    if (!trimmed) {
      setHits([]);
      setTruncated(false);
      setError(null);
      return;
    }
    const seq = ++seqRef.current;
    setLoading(true);
    try {
      const res = await reflect_search_files(trimmed, null, 200);
      if (seq !== seqRef.current) return; // stale
      setHits(res?.hits ?? []);
      setTruncated(res?.truncated ?? false);
      setError(null);
    } catch (e) {
      if (seq !== seqRef.current) return; // stale
      setHits([]);
      setTruncated(false);
      setError((e as Error).message);
    } finally {
      if (seq === seqRef.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (debounceRef.current) window.clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      void runSearch(query);
    }, 200);
    return () => {
      if (debounceRef.current) window.clearTimeout(debounceRef.current);
    };
  }, [query, runSearch]);

  return (
    <PageShell
      icon={Search}
      title="Find in files"
      subtitle="Search file contents across the active workspace. Skips .git, node_modules, target, dist, build."
      width="lg"
    >
      <Card level="flat" padding="lg">
        <div className={s.searchBox}>
          <Icon icon={Search} size={14} />
          <input
            className={s.input}
            placeholder="Type to search…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoFocus
            aria-label="Find in files query"
          />
          {query && (
            <button
              type="button"
              onClick={() => setQuery('')}
              aria-label="Clear search"
              className={s.clear}
            >
              <Icon icon={X} size={14} />
            </button>
          )}
        </div>

        {loading && (
          <div className={s.status}>
            <Spinner size={14} /> Searching…
          </div>
        )}

        {!loading && error && (
          <EmptyState
            size="sm"
            icon={<Icon icon={Search} size={20} />}
            title="Search failed"
            description={error}
          />
        )}

        {!loading && !error && query && hits.length === 0 && (
          <EmptyState
            size="sm"
            icon={<Icon icon={Search} size={20} />}
            title="No matches"
            description={`No files contain "${query}".`}
          />
        )}

        {!loading && hits.length > 0 && (
          <>
            <div className={s.summary}>
              <Badge variant="info">
                {hits.length} hit{hits.length === 1 ? '' : 's'}
              </Badge>
              {truncated && <Badge variant="warning">truncated at 200</Badge>}
            </div>
            <ul className={s.list}>
              {hits.map((h, idx) => (
                <li key={`${h.path}-${h.line}-${idx}`}>
                  <button
                    type="button"
                    className={s.hitButton}
                    onClick={() =>
                      navigate({
                        to: '/files',
                        search: { path: h.path, line: h.line } as never,
                      } as never)
                    }
                  >
                    <div className={s.hitPath}>
                      <code className={s.path}>{h.path}</code>
                      <Badge variant="neutral">L{h.line}</Badge>
                    </div>
                    <pre className={s.context}>{h.context}</pre>
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
      </Card>
    </PageShell>
  );
}
