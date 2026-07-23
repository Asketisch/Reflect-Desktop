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
  useMemo,
  type KeyboardEvent,
  type ChangeEvent,
} from 'react';
import { ArrowUp, SlashSquare, Image as ImageIcon, FileText, AtSign } from 'lucide-react';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { SlashPopup } from '@/features/composer/SlashPopup';
import { TIER_A_TOOLBAR } from '@/features/composer/slashCommands';
import { dispatch } from '@/features/composer/slashEngine';
import { AttachmentBar } from '@/features/composer/AttachmentBar';
import { useAttachments } from '@/features/composer/useAttachments';
import { MentionPicker } from '@/features/composer/MentionPicker';
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
  const [mentionOpen, setMentionOpen] = useState(false);
  const ref = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const att = useAttachments();

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
    if (!value && att.attachments.length === 0) return;
    setBusy(true);
    setText('');
    setSlashVisible(false);
    try {
      const parsed = dispatch(value, { activeSessionId: activeId, now: () => new Date() });
      if ('kind' in parsed) {
        if (parsed.kind === 'submit_with_submission' && parsed.submission) {
          const re = /^(\/\S+)(?:\s+(.*))?$/;
          const m = re.exec(value.trim());
          const args = m && m[2] ? m[2].split(/\s+/) : [];
          await dispatchSubmission(parsed.submission, args);
        } else if (parsed.kind === 'reject') {
          pushToast({ kind: 'error', message: parsed.message ?? 'Command rejected.' });
        } else if (parsed.kind === 'no-op') {
          pushToast({ kind: 'info', message: parsed.message ?? 'Command handled.' });
        }
      } else {
        // Plain text — submit to agent (B5: with attachments).
        if (att.attachments.length > 0) {
          const items = att.toUserInputItems();
          await useAgentStore.getState().submitItems([
            ...(value ? [{ type: 'text' as const, text: value }] : []),
            ...items,
          ]);
        } else {
          await submit(value);
        }
      }
      att.clear();
    } catch (e) {
      console.error(e);
      pushToast({ kind: 'error', message: e instanceof Error ? e.message : String(e) });
    } finally {
      setBusy(false);
      ref.current?.focus();
    }
  }, [text, activeId, submit, dispatchSubmission, pushToast, att]);

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

  const canSend = !busy && (!!text.trim() || att.attachments.length > 0);
  const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);
  const sendHint = isMac ? 'Send (⌘↵)' : 'Send (Ctrl+↵)';

  // B5: @mention query — extract "@xxx" prefix at cursor.
  const mentionQuery = useMemo(() => {
    const m = text.match(/(?:^|\s)@(\w*)$/);
    return m ? m[1] : '';
  }, [text]);

  // Hidden file inputs for inline image / file picking (Tauri webview).
  const onPickImage = useCallback(
    (files: FileList | null) => {
      if (!files) return;
      Array.from(files).forEach((file) => {
        if (!file.type.startsWith('image/')) return;
        const reader = new FileReader();
        reader.onload = () => {
          if (typeof reader.result === 'string') {
            att.addInlineImage(reader.result, file.type);
          }
        };
        reader.readAsDataURL(file);
      });
    },
    [att],
  );

  const onPickFile = useCallback(
    (files: FileList | null) => {
      if (!files) return;
      // For now store the file name as a placeholder path; real workspace-relative
      // paths require tauri-plugin-dialog (deferred). We still surface the
      // attachment to the user.
      Array.from(files).forEach((file) => {
        att.addLocalImage(`(file) ${file.name}`);
      });
    },
    [att],
  );

  return (
    <div className={s.wrap}>
      <SlashPopup query={slashQuery} visible={slashVisible} onSelect={onSlashSelect} />
      <MentionPicker
        visible={mentionOpen && mentionQuery !== undefined}
        query={mentionQuery}
        onPick={(name) => {
          att.addSkillMention(name);
          setMentionOpen(false);
        }}
        onClose={() => setMentionOpen(false)}
      />

      {/* Hidden file inputs */}
      <input
        ref={fileInputRef}
        type="file"
        multiple
        style={{ display: 'none' }}
        data-testid="composer-file-input"
        onChange={(e) => onPickFile(e.target.files)}
      />
      <input
        ref={imageInputRef}
        type="file"
        accept="image/*"
        multiple
        style={{ display: 'none' }}
        data-testid="composer-image-input"
        onChange={(e) => onPickImage(e.target.files)}
      />

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
        <AttachmentBar attachments={att.attachments} onRemove={att.remove} />
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
            <Tooltip label="Attach file" side="top">
              <IconButton
                label="Attach file"
                size="sm"
                onClick={() => fileInputRef.current?.click()}
                data-testid="composer-attach-file"
              >
                <Icon icon={FileText} size={14} />
              </IconButton>
            </Tooltip>
            <Tooltip label="Attach image" side="top">
              <IconButton
                label="Attach image"
                size="sm"
                onClick={() => imageInputRef.current?.click()}
                data-testid="composer-attach-image"
              >
                <Icon icon={ImageIcon} size={14} />
              </IconButton>
            </Tooltip>
            <Tooltip label="Mention skill" side="top">
              <IconButton
                label="Mention skill"
                size="sm"
                onClick={() => {
                  setText((v) => `${v} @`);
                  setMentionOpen(true);
                  ref.current?.focus();
                }}
                data-testid="composer-attach-mention"
              >
                <Icon icon={AtSign} size={14} />
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
