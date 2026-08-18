/**
 * SlashPopup —— `/` 命令候选弹层（CSS Modules 版）。
 *
 * 契约（SlashPopup.test.tsx）：
 *   - 渲染 button 元素，文本含 `/<name>`
 *   - 点击 button 调 onSelect(name)
 *   - visible=false / 无匹配时返回 null
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { SLASH_COMMANDS } from './slashCommands';
import { useI18n } from '@/utils/i18n';
import s from './SlashPopup.module.css';

interface Props {
  query: string;
  onSelect: (cmd: string) => void;
  visible: boolean;
}

export function SlashPopup({ query, onSelect, visible }: Props) {
  const { t } = useI18n();
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

  useEffect(() => {
    setActiveIdx(0);
  }, [query]);

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
    // 注意：键盘导航目前仅支持鼠标驱动——textarea 在弹层打开时保持焦点，
    // 因此弹层的 keydown 处理器不会因真实的按键而触发。点击选择（第 71 行）
    // 仍然有效。正确的修复方案是在打开时将焦点转移到弹层。
    node.addEventListener('keydown', onKey);
    return () => node.removeEventListener('keydown', onKey);
  }, [visible, filtered, activeIdx, onSelect]);

  if (!visible || filtered.length === 0) return null;

  return (
    <div ref={listRef} tabIndex={-1} className={s.popup} role="listbox">
      {filtered.map((c, i) => (
        <button
          type="button"
          key={c.name}
          onMouseEnter={() => setActiveIdx(i)}
          onClick={() => onSelect(c.name)}
          className={s.item}
          data-active={i === activeIdx || undefined}
          role="option"
          aria-selected={i === activeIdx}
        >
          <span className={s.name}>/{c.name}</span>
          <span className={s.summary}>{t(c.summaryKey as Parameters<typeof t>[0])}</span>
        </button>
      ))}
    </div>
  );
}
