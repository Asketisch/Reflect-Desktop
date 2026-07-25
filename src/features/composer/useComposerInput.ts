import { useCallback, type ChangeEvent, type KeyboardEvent, type RefObject } from 'react';
import type { PromptHistoryApi } from './usePromptHistory';

const SLASH_QUERY = /(^|\s)(\/\w*)$/;

interface UseComposerInputOptions {
  text: string;
  setText: (next: string) => void;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  slashVisible: boolean;
  setSlashVisible: (visible: boolean) => void;
  setSlashQuery: (query: string) => void;
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
  history,
  submit,
}: UseComposerInputOptions) {
  const onChange = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
    const value = event.target.value;
    setText(value);
    if (history.isBrowsing()) history.resetNavigation();

    const match = SLASH_QUERY.exec(value);
    setSlashVisible(Boolean(match));
    setSlashQuery(match?.[2] ?? '');

    event.target.style.height = 'auto';
    event.target.style.height = `${Math.min(event.target.scrollHeight, 200)}px`;
  }, [history, setSlashQuery, setSlashVisible, setText]);

  const onKeyDown = useCallback((event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (isCompositionEvent(event)) return;

    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      submit();
      return;
    }

    if (event.key === 'Escape' && slashVisible) {
      setSlashVisible(false);
      return;
    }

    if (slashVisible || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;

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
  }, [history, setSlashVisible, setText, slashVisible, submit, text, textareaRef]);

  return { onChange, onKeyDown };
}
