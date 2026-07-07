/**
 * M1.5 SlashPopup —— 输入 `/` 后弹出命令候选。
 *
 * 当前 stub 实现:选中后把 `/<cmd>` 文本填回 Composer。
 * M2.x 把每个命令绑定到对应 Tauri command / Reflect API。
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { SLASH_COMMANDS } from './slashCommands';

interface Props {
  /** 当前已经输入的 slash 文本,如 "/co" */
  query: string;
  onSelect: (cmd: string) => void;
  visible: boolean;
}

export function SlashPopup({ query, onSelect, visible }: Props) {
  const listRef = useRef<HTMLDivElement>(null);
  const [activeIdx, setActiveIdx] = useState(0);

  const filtered = useMemo(() => {
    const q = query.toLowerCase().replace(/^\//, '');
    if (!q) return SLASH_COMMANDS;
    return SLASH_COMMANDS.filter(
      (c) =>
        c.name.toLowerCase().startsWith(q) ||
        c.aliases?.some((a) => a.toLowerCase().startsWith(q)),
    );
  }, [query]);

  // query 改变重置选中
  useEffect(() => {
    setActiveIdx(0);
  }, [query]);

  // 键盘导航
  useEffect(() => {
    if (!visible) return;
    const node = listRef.current;
    if (!node) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIdx((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIdx((i) => Math.max(i - 1, 0));
      } else if (e.key === 'Enter' || e.key === 'Tab') {
        const c = filtered[activeIdx];
        if (c) {
          e.preventDefault();
          onSelect(c.name);
        }
      }
    };
    node.addEventListener('keydown', onKey);
    return () => node.removeEventListener('keydown', onKey);
  }, [visible, filtered, activeIdx, onSelect]);

  if (!visible || filtered.length === 0) return null;

  return (
    <div
      ref={listRef}
      tabIndex={-1}
      style={{
        position: 'absolute',
        bottom: '100%',
        left: 0,
        right: 0,
        marginBottom: 8,
        background: 'white',
        border: '1px solid #ddd',
        borderRadius: 6,
        maxHeight: 240,
        overflowY: 'auto',
        boxShadow: '0 4px 16px rgba(0,0,0,0.08)',
        zIndex: 10,
      }}
    >
      {filtered.map((c, i) => (
        <button
          type="button"
          key={c.name}
          onMouseEnter={() => setActiveIdx(i)}
          onClick={() => onSelect(c.name)}
          style={{
            display: 'block',
            width: '100%',
            textAlign: 'left',
            padding: '6px 10px',
            border: 'none',
            background: i === activeIdx ? '#eef2ff' : 'transparent',
            cursor: 'pointer',
          }}
        >
          <div style={{ fontWeight: 500 }}>/{c.name}</div>
          <div style={{ fontSize: 11, color: '#666' }}>{c.summary}</div>
        </button>
      ))}
    </div>
  );
}
