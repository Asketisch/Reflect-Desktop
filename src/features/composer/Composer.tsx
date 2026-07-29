import { useCallback, useMemo, useRef, useState } from 'react';
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
import { useI18n } from '@/utils/i18n';
import s from './Composer.module.css';

export function Composer() {
  const { t } = useI18n();
  const [text, setText] = useState('');
  const [slashVisible, setSlashVisible] = useState(false);
  const [slashQuery, setSlashQuery] = useState('');
  const [busy, setBusy] = useState(false);
  const [mentionOpen, setMentionOpen] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const attachments = useAttachments();
  const history = usePromptHistory('default');

  const focus = useCallback(() => textareaRef.current?.focus(), []);
  const doSubmit = useComposerSubmission({
    text,
    attachments,
    history,
    setText,
    setBusy,
    setSlashVisible,
    focus,
  });
  const { onChange, onKeyDown } = useComposerInput({
    text,
    setText,
    textareaRef,
    slashVisible,
    setSlashVisible,
    setSlashQuery,
    history,
    submit: () => void doSubmit(),
  });

  const onSlashSelect = useCallback((command: string) => {
    setText((previous) => {
      // Strip any existing `/<word>` token (and any args the user typed after
      // it) so a click on a popup entry always yields a clean `/<command> `.
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

  const mentionQuery = useMemo(() => {
    const match = text.match(/(?:^|\s)@(\w*)$/);
    return match ? match[1] : '';
  }, [text]);

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

  const canSend = !busy && (Boolean(text.trim()) || attachments.attachments.length > 0);
  const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);
  const sendHint = isMac ? t('composer.sendHintMac') : t('composer.sendHintOther');

  return (
    <div className={s.wrap}>
      <SlashPopup query={slashQuery} visible={slashVisible} onSelect={onSlashSelect} />
      <MentionPicker
        visible={mentionOpen}
        query={mentionQuery}
        onPick={(name) => {
          attachments.addSkillMention(name);
          setMentionOpen(false);
        }}
        onClose={() => setMentionOpen(false)}
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
            <Tooltip label={t('composer.toolbar.mentionSkill')} side="top">
              <IconButton
                label={t('composer.toolbar.mentionSkill')}
                size="sm"
                onClick={() => {
                  setText((value) => `${value} @`);
                  setMentionOpen(true);
                  focus();
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
