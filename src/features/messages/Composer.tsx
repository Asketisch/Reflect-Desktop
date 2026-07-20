/**
 * Composer —— 卡片式输入区（CSS Modules 版）。
 *
 *   - 输入 `/` 触发 SlashPopup
 *   - Tier A toolbar: 9 个常用 slash 命令
 *   - Cmd/Ctrl+Enter 提交（IME 安全）
 *   - 卡片容器 + 底部工具栏 + Send IconButton（ArrowUp 图标）
 */
import {
  useState,
  useRef,
  useCallback,
  type KeyboardEvent,
  type ChangeEvent,
} from 'react';
import { ArrowUp, SlashSquare } from 'lucide-react';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { SlashPopup } from '@/features/composer/SlashPopup';
import { TIER_A_TOOLBAR } from '@/features/composer/slashCommands';
import { useAgent } from '@/services/agent';
import s from './Composer.module.css';

export function Composer() {
  const { submit } = useAgent();
  const [text, setText] = useState('');
  const [slashVisible, setSlashVisible] = useState(false);
  const [slashQuery, setSlashQuery] = useState('');
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLTextAreaElement>(null);

  const onChange = useCallback((e: ChangeEvent<HTMLTextAreaElement>) => {
    const v = e.target.value;
    setText(v);
    const slashRe = new RegExp('(^|\\s)(\\/\\w*)$');
    const m = slashRe.exec(v);
    if (m) {
      setSlashVisible(true);
      setSlashQuery(m[2]);
    } else {
      setSlashVisible(false);
      setSlashQuery('');
    }
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 200) + 'px';
  }, []);

  const onKey = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        void doSubmit();
      } else if (e.key === 'Escape' && slashVisible) {
        setSlashVisible(false);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [text, slashVisible],
  );

  const doSubmit = useCallback(async () => {
    const value = text.trim();
    if (!value) return;
    setBusy(true);
    setText('');
    setSlashVisible(false);
    try {
      await submit(value);
    } catch (e) {
      console.error(e);
    } finally {
      setBusy(false);
      ref.current?.focus();
    }
  }, [text, submit]);

  const onSlashSelect = useCallback((cmd: string) => {
    setText((prev) => {
      const replaced = prev.replace(/(\/\w*)$/, `/${cmd} `);
      requestAnimationFrame(() => {
        const el = ref.current;
        if (el) {
          el.focus();
          el.selectionStart = el.selectionEnd = replaced.length;
        }
      });
      return replaced;
    });
    setSlashVisible(false);
  }, []);

  const onToolbarClick = (cmd: string) => {
    setText(`/${cmd} `);
    setSlashVisible(false);
    ref.current?.focus();
  };

  const canSend = !busy && !!text.trim();
  const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);
  const sendHint = isMac ? 'Send (⌘↵)' : 'Send (Ctrl+↵)';

  return (
    <div className={s.wrap}>
      <SlashPopup query={slashQuery} visible={slashVisible} onSelect={onSlashSelect} />

      <div className={s.toolbar}>
        {TIER_A_TOOLBAR.map((c) => (
          <button
            key={c.name}
            type="button"
            onClick={() => onToolbarClick(c.name)}
            title={c.summary}
            className={s.toolBtn}
          >
            /{c.name}
          </button>
        ))}
      </div>

      <div className={s.card}>
        <textarea
          ref={ref}
          rows={1}
          value={text}
          disabled={busy}
          onChange={onChange}
          onKeyDown={onKey}
          placeholder="Ask Reflect anything…  (type / for commands)"
          aria-label="Message Reflect"
          className={s.textarea}
          autoFocus
        />
        <div className={s.bottomBar}>
          <div className={s.bottomLeft}>
            <Tooltip label="Slash commands" side="top">
              <IconButton
                label="Slash commands"
                size="sm"
                onClick={() => {
                  setText((v) => (v.startsWith('/') ? v : '/' + v));
                  ref.current?.focus();
                }}
              >
                <Icon icon={SlashSquare} size={14} />
              </IconButton>
            </Tooltip>
          </div>
          <div className={s.bottomRight}>
            <Tooltip label={sendHint} side="top">
              <IconButton
                label={sendHint}
                variant={canSend ? 'primary' : 'default'}
                size="md"
                disabled={!canSend}
                onClick={() => void doSubmit()}
              >
                <Icon icon={ArrowUp} size={16} />
              </IconButton>
            </Tooltip>
          </div>
        </div>
      </div>
    </div>
  );
}
