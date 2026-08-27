import { useCallback, type ChangeEvent, type KeyboardEvent, type RefObject } from 'react';
import type { PromptHistoryApi } from './usePromptHistory';

const SLASH_QUERY = /(^|\s)(\/\w*)$/;
/**
 * `@<query>` 触发器：行首或空白后的 `@` + 非 `@` 非空白字符序列。
 * - 不允许 `@` 出现在 query 内（避免 `@@foo` 解析歧义）
 * - 不允许空白字符在 query 内（与 `/` slash 行为一致，避免被空格提前结束）
 */
const MENTION_QUERY = /(^|\s)@([^@\s]*)$/;

export interface UseComposerInputOptions {
  text: string;
  setText: (next: string) => void;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  slashVisible: boolean;
  setSlashVisible: (visible: boolean) => void;
  setSlashQuery: (query: string) => void;
  /**
   * v1.x：`@` 弹层联动（取代 Composer 内的 useMemo 计算）。
   * - `mentionVisible = true` 时 MentionPicker 渲染；
   * - `mentionQuery` 是 `@` 后待匹配的字符串（如 `"RE"` for "读 @RE"）。
   */
  mentionVisible: boolean;
  setMentionVisible: (visible: boolean) => void;
  setMentionQuery: (query: string) => void;
  history: PromptHistoryApi;
  submit: () => void;
}

function isCompositionEvent(event: KeyboardEvent<HTMLTextAreaElement>): boolean {
  return event.nativeEvent.isComposing || event.keyCode === 229;
}

export function useComposerInput({
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
  submit,
}: UseComposerInputOptions) {
  const onChange = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
    const value = event.target.value;
    setText(value);
    if (history.isBrowsing()) history.resetNavigation();

    const slash = SLASH_QUERY.exec(value);
    setSlashVisible(Boolean(slash));
    setSlashQuery(slash?.[2] ?? '');

    const mention = MENTION_QUERY.exec(value);
    setMentionVisible(Boolean(mention));
    setMentionQuery(mention?.[2] ?? '');

    event.target.style.height = 'auto';
    event.target.style.height = `${Math.min(event.target.scrollHeight, 200)}px`;
  }, [history, setMentionQuery, setMentionVisible, setSlashQuery, setSlashVisible, setText]);

  const onKeyDown = useCallback((event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (isCompositionEvent(event)) return;

    // Enter 默认发送(meta/ctrl+Enter 同路径兼容旧习惯);
    // Shift+Enter 走浏览器默认行为插入换行。
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
      return;
    }

    if (event.key === 'Escape') {
      if (slashVisible) {
        setSlashVisible(false);
        return;
      }
      if (mentionVisible) {
        setMentionVisible(false);
        return;
      }
    }

    if (slashVisible || mentionVisible || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;

    const element = textareaRef.current;
    if (!element) return;
    const start = element.selectionStart ?? 0;
    const end = element.selectionEnd ?? 0;
    const onFirstLine = !element.value.slice(0, start).includes('\n');
    const onLastLine = !element.value.slice(end).includes('\n');

    if (event.key === 'ArrowUp' && onFirstLine) {
      if (history.recallPrev(text, setText)) event.preventDefault();
    } else if (event.key === 'ArrowDown' && onLastLine && history.isBrowsing()) {
      if (history.recallNext(setText)) event.preventDefault();
    }
  }, [history, mentionVisible, setMentionVisible, setSlashVisible, setText, slashVisible, submit, text, textareaRef]);

  return { onChange, onKeyDown };
}
