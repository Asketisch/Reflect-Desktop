import { useCallback, useRef, useState } from 'react';
import { ArrowUp, SlashSquare, Image as ImageIcon, FileText, AtSign } from 'lucide-react';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { SlashPopup } from './SlashPopup';
import { TIER_A_TOOLBAR } from './slashCommands';
import { AttachmentBar } from './AttachmentBar';
import { useAttachments } from './useAttachments';
import { MentionPicker } from './MentionPicker';
import { usePromptHistory } from './usePromptHistory';
import { useComposerInput } from './useComposerInput';
import { useComposerSubmission } from './useComposerSubmission';
import { useCurrentWorkspace } from '@/features/shell/hooks/useCurrentWorkspace';
import { useI18n } from '@/utils/i18n';
import s from './Composer.module.css';

export function Composer() {
  const { t } = useI18n();
  const [text, setText] = useState('');
  const [slashVisible, setSlashVisible] = useState(false);
  const [slashQuery, setSlashQuery] = useState('');
  const [busy, setBusy] = useState(false);
  const [mentionVisible, setMentionVisible] = useState(false);
  const [mentionQuery, setMentionQuery] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const attachments = useAttachments();
  const history = usePromptHistory('default');
  const { currentWorkspace } = useCurrentWorkspace();

  const focus = useCallback(() => textareaRef.current?.focus(), []);
  const doSubmit = useComposerSubmission({
    text,
    attachments,
    history,
    setText,
    setBusy,
    setSlashVisible,
    focus,
    currentWorkspace,
  });
  const { onChange, onKeyDown } = useComposerInput({
    text,
    setText,
    textareaRef,
    slashVisible,
    setSlashVisible,
    setSlashQuery,
    mentionVisible,
    setMentionVisible,
    setMentionQuery,
    history,
    submit: () => void doSubmit(),
  });

  const onSlashSelect = useCallback((command: string) => {
    setText((previous) => {
      // 清除已有的 `/<word>` token（以及用户输入的参数），
      // 使点击弹层条目始终产生干净的 `/<command> `。
      const replaced = previous.replace(/\/\w*(?:\s+.*)?$/, `/${command} `);
      requestAnimationFrame(() => {
        const element = textareaRef.current;
        if (element) {
          element.focus();
          element.selectionStart = element.selectionEnd = replaced.length;
        }
      });
      return replaced;
    });
    setSlashVisible(false);
  }, []);

  const onPickImage = useCallback((files: FileList | null) => {
    if (!files) return;
    Array.from(files).forEach((file) => {
      if (!file.type.startsWith('image/')) return;
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === 'string') attachments.addInlineImage(reader.result, file.type);
      };
      reader.readAsDataURL(file);
    });
  }, [attachments]);

  const onPickFile = useCallback((files: FileList | null) => {
    if (!files) return;
    Array.from(files).forEach((file) => attachments.addLocalImage(`(file) ${file.name}`));
  }, [attachments]);

  const onPickMention = useCallback(
    (relPath: string) => {
      // 1. 清除 textarea 末尾的 `@<query>` 片段（保留前导空白）
      setText((prev) => prev.replace(/(^|\s)@[^@\s]*$/, '$1'));
      // 2. 把文件加进 attachments（待发列表）
      attachments.addFileMention(relPath);
      setMentionVisible(false);
      focus();
    },
    [attachments, focus],
  );

  const onMentionButton = useCallback(() => {
    // 工具栏 `@` 按钮：在末尾追加 `@`（必要时补前导空格）并**直接**打开弹层。
    // 直接 setState 不触发 onChange，MENTION_QUERY 不会重算 ——
    // 因此这里显式 setMentionVisible(true)（后续输入会由 onChange 接管）。
    setText((prev) => {
      const needsSpace = prev.length > 0 && !/\s$/.test(prev);
      const next = `${prev}${needsSpace ? ' ' : ''}@`;
      requestAnimationFrame(() => {
        const el = textareaRef.current;
        if (el) {
          el.focus();
          el.selectionStart = el.selectionEnd = next.length;
        }
      });
      return next;
    });
    setMentionVisible(true);
    setMentionQuery('');
  }, []);

  const workspaceLabel = currentWorkspace
    ? `@ ${currentWorkspace.split('/').filter(Boolean).pop() ?? currentWorkspace}`
    : undefined;

  const canSend = !busy && (Boolean(text.trim()) || attachments.attachments.length > 0);
  const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);
  const sendHint = isMac ? t('composer.sendHintMac') : t('composer.sendHintOther');

  return (
    <div className={s.wrap}>
      <SlashPopup query={slashQuery} visible={slashVisible} onSelect={onSlashSelect} />
      <MentionPicker
        visible={mentionVisible}
        query={mentionQuery}
        workspaceLabel={workspaceLabel}
        onPickFile={onPickMention}
        onClose={() => setMentionVisible(false)}
      />

      <input
        ref={fileInputRef}
        type="file"
        multiple
        hidden
        data-testid="composer-file-input"
        onChange={(event) => onPickFile(event.target.files)}
      />
      <input
        ref={imageInputRef}
        type="file"
        accept="image/*"
        multiple
        hidden
        data-testid="composer-image-input"
        onChange={(event) => onPickImage(event.target.files)}
      />

      <div className={s.toolbar}>
        {TIER_A_TOOLBAR.map((command) => (
          <button
            key={command.name}
            type="button"
            onClick={() => {
              setText((prev) => `${prev}${prev && !prev.endsWith(' ') ? ' ' : ''}/${command.name} `);
              setSlashVisible(false);
              focus();
            }}
            title={t(command.summaryKey as Parameters<typeof t>[0])}
            className={s.toolBtn}
          >
            /{command.name}
          </button>
        ))}
      </div>

      <div className={s.card}>
        <AttachmentBar attachments={attachments.attachments} onRemove={attachments.remove} />
        <textarea
          ref={textareaRef}
          rows={1}
          value={text}
          disabled={busy}
          onChange={onChange}
          onKeyDown={onKeyDown}
          placeholder={t('composer.placeholder')}
          aria-label={t('composer.ariaLabel')}
          className={s.textarea}
          autoFocus
        />
        <div className={s.bottomBar}>
          <div className={s.bottomLeft}>
            <Tooltip label={t('composer.toolbar.slashCommands')} side="top">
              <IconButton
                label={t('composer.toolbar.slashCommands')}
                size="sm"
                onClick={() => {
                  setText((value) => (value.startsWith('/') ? value : `/${value}`));
                  focus();
                }}
              >
                <Icon icon={SlashSquare} size={14} />
              </IconButton>
            </Tooltip>
            <Tooltip label={t('composer.toolbar.attachFile')} side="top">
              <IconButton
                label={t('composer.toolbar.attachFile')}
                size="sm"
                onClick={() => fileInputRef.current?.click()}
                data-testid="composer-attach-file"
              >
                <Icon icon={FileText} size={14} />
              </IconButton>
            </Tooltip>
            <Tooltip label={t('composer.toolbar.attachImage')} side="top">
              <IconButton
                label={t('composer.toolbar.attachImage')}
                size="sm"
                onClick={() => imageInputRef.current?.click()}
                data-testid="composer-attach-image"
              >
                <Icon icon={ImageIcon} size={14} />
              </IconButton>
            </Tooltip>
            <Tooltip label={t('composer.toolbar.mentionFile')} side="top">
              <IconButton
                label={t('composer.toolbar.mentionFile')}
                size="sm"
                onClick={onMentionButton}
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
