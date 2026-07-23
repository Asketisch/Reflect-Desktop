/**
 * Composer —— 卡片式输入区（CSS Modules 版）。
 *
 *   - 输入 `/` 触发 SlashPopup
 *   - Tier A toolbar: 9 个常用 slash 命令
 *   - Cmd/Ctrl+Enter 提交（IME 安全）
 *   - 卡片容器 + 底部工具栏 + Send IconButton（ArrowUp 图标）
 *   - B4: 提交时通过 slashEngine 解析 → 路由到对应 Submission
 *     (compact / enter_plan_mode / exit_plan_mode / set_effort / set_permission_mode / interrupt / export_session / rename_session)
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
import { dispatch } from '@/features/composer/slashEngine';
import { useAgent } from '@/services/agent';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import {
  reflect_compact,
  reflect_interrupt,
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_export_session,
} from '@/utils/tauri';
import { useAgentStore } from '@/stores/agentStore';
import s from './Composer.module.css';

export function Composer() {
  const { submit } = useAgent();
  const { activeId } = useActiveSession();
  const { rename } = useSessions();
  const pushToast = useAgentStore((s) => s.pushToast);
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

  /** Dispatch a Submission by its string kind (B4: slash engine output). */
  const dispatchSubmission = useCallback(
    async (kind: string, args: string[]): Promise<void> => {
      switch (kind) {
        case 'compact':
          await reflect_compact();
          pushToast({ kind: 'info', message: 'Compact requested.' });
          break;
        case 'enter_plan_mode':
          await reflect_enter_plan_mode(args.join(' ') || 'unspecified');
          break;
        case 'exit_plan_mode':
          await reflect_exit_plan_mode();
          break;
        case 'set_effort': {
          const level = (args[0] ?? '').toLowerCase();
          await reflect_set_effort(level);
          break;
        }
        case 'set_permission_mode': {
          const m = (args[0] ?? '').toLowerCase();
          await reflect_set_permission_mode(m);
          break;
        }
        case 'interrupt':
          await reflect_interrupt();
          break;
        case 'export_session':
          if (activeId) {
            const path = await reflect_export_session(activeId);
            pushToast({ kind: 'info', message: `Exported → ${path ?? '(no path)'}` });
          }
          break;
        case 'rename_session': {
          const name = args.join(' ').trim();
          if (activeId && name) await rename(activeId, name);
          break;
        }
        default:
          pushToast({ kind: 'warn', message: `Unhandled slash submission: ${kind}` });
      }
    },
    [activeId, rename, pushToast],
  );

  const doSubmit = useCallback(async () => {
    const value = text.trim();
    if (!value) return;
    setBusy(true);
    setText('');
    setSlashVisible(false);
    try {
      const result = dispatch(value, { activeSessionId: activeId, now: () => new Date() });
      if ('kind' in result) {
        if (result.kind === 'submit_with_submission' && result.submission) {
          await dispatchSubmission(result.submission, ('args' in result ? (result as { args?: string[] }).args ?? [] : []));
        } else if (result.kind === 'reject') {
          pushToast({ kind: 'error', message: result.message ?? 'Command rejected.' });
        } else if (result.kind === 'no-op') {
          pushToast({ kind: 'info', message: result.message ?? 'Command handled.' });
        }
      } else {
        // Plain text — submit to agent.
        await submit(value);
      }
    } catch (e) {
      console.error(e);
      pushToast({ kind: 'error', message: e instanceof Error ? e.message : String(e) });
    } finally {
      setBusy(false);
      ref.current?.focus();
    }
  }, [text, activeId, submit, dispatchSubmission, pushToast]);

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
