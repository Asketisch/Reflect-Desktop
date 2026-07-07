/**
 * M1.5 Composer —— auto-grow textarea + SlashPopup + Tier A toolbar。
 *   - 输入 `/` 触发 SlashPopup,选择命令 → 把 `/cmd` 写入文本框等待回车
 *   - Tier A toolbar (9 按钮):快速触发常用 slash 命令
 *   - Cmd/Ctrl+Enter 提交(IME 安全)
 *
 * M2.x 升级:@提及文件、图像附件、外部编辑器。
 */
import {
  useState,
  useRef,
  useCallback,
  type KeyboardEvent,
  type ChangeEvent,
} from 'react';
import { SlashPopup } from '@/features/composer/SlashPopup';
import { TIER_A_TOOLBAR } from '@/features/composer/slashCommands';
import { useAgent } from '@/services/agent';

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
    // 检测当前是否在 `/xxx` 上下文 —— 简化:只匹配文本末尾 `/` 后无空格的 token
    const slashRe = new RegExp('(^|\\s)(\\/\\w*)$');
    const m = slashRe.exec(v);
    if (m) {
      setSlashVisible(true);
      setSlashQuery(m[2]);
    } else {
      setSlashVisible(false);
      setSlashQuery('');
    }
    // auto-grow 1→6 行
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 200) + 'px';
  }, []);

  const onKey = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        doSubmit();
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
      // 移动 cursor 到末尾
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

  return (
    <div style={{ position: 'relative' }}>
      <SlashPopup
        query={slashQuery}
        visible={slashVisible}
        onSelect={onSlashSelect}
      />
      <div
        style={{
          display: 'flex',
          gap: 4,
          marginBottom: 6,
          flexWrap: 'wrap',
        }}
      >
        {TIER_A_TOOLBAR.map((c) => (
          <button
            key={c.name}
            type="button"
            onClick={() => onToolbarClick(c.name)}
            title={c.summary}
            style={{
              padding: '4px 8px',
              fontSize: 11,
              border: '1px solid #ddd',
              borderRadius: 4,
              background: 'white',
              cursor: 'pointer',
            }}
          >
            {c.name}
          </button>
        ))}
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          doSubmit();
        }}
        style={{ display: 'flex', gap: 8 }}
      >
        <textarea
          ref={ref}
          rows={1}
          value={text}
          disabled={busy}
          onChange={onChange}
          onKeyDown={onKey}
          placeholder="Say hi (type / for commands, Cmd/Ctrl+Enter to send)"
          style={{
            flex: 1,
            padding: 8,
            fontSize: 14,
            fontFamily: 'inherit',
            resize: 'none',
            maxHeight: 200,
            minHeight: 36,
          }}
        />
        <button type="submit" disabled={busy || !text.trim()}>
          Send
        </button>
      </form>
    </div>
  );
}
