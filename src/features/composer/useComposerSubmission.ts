import { useCallback } from 'react';
import { dispatch } from './slashEngine';
import type { UseAttachmentsResult } from './useAttachments';
import { useAgent } from '@/services/agent';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { useI18n } from '@/utils/i18n';
import {
  reflect_compact,
  reflect_interrupt,
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_export_session,
} from '@/utils/commands';
import { useAgentStore } from '@/stores/agentStore';
import type { PromptHistoryApi } from './usePromptHistory';

interface UseComposerSubmissionOptions {
  text: string;
  attachments: UseAttachmentsResult;
  history: PromptHistoryApi;
  setText: (text: string) => void;
  setBusy: (busy: boolean) => void;
  setSlashVisible: (visible: boolean) => void;
  focus: () => void;
}

export function useComposerSubmission({
  text,
  attachments,
  history,
  setText,
  setBusy,
  setSlashVisible,
  focus,
}: UseComposerSubmissionOptions) {
  const { submit } = useAgent();
  const { activeId } = useActiveSession();
  const { rename } = useSessions();
  const pushToast = useAgentStore((state) => state.pushToast);
  const { t } = useI18n();

  const dispatchSubmission = useCallback(async (kind: string, args: string[]) => {
    switch (kind) {
      case 'compact':
        await reflect_compact();
        pushToast({ kind: 'info', message: t('composer.compactRequested') });
        break;
      case 'enter_plan_mode':
        await reflect_enter_plan_mode(args.join(' ') || 'unspecified');
        break;
      case 'exit_plan_mode':
        await reflect_exit_plan_mode();
        break;
      case 'set_effort':
        await reflect_set_effort((args[0] ?? '').toLowerCase());
        break;
      case 'set_permission_mode':
        await reflect_set_permission_mode((args[0] ?? '').toLowerCase());
        break;
      case 'interrupt':
        await reflect_interrupt();
        break;
      case 'export_session':
        if (activeId) {
          const path = await reflect_export_session(activeId);
          pushToast({
            kind: 'info',
            message: path ? t('composer.exported', { path }) : t('composer.exportedNoPath'),
          });
        }
        break;
      case 'rename_session': {
        const name = args.join(' ').trim();
        if (activeId && name) await rename(activeId, name);
        break;
      }
      default:
        pushToast({ kind: 'warn', message: t('composer.unhandledSlash', { kind }) });
    }
  }, [activeId, pushToast, rename, t]);

  return useCallback(async () => {
    const value = text.trim();
    if (!value && attachments.attachments.length === 0) return;

    setBusy(true);
    setText('');
    setSlashVisible(false);
    history.resetNavigation();
    try {
      const parsed = dispatch(value, { activeSessionId: activeId, now: () => new Date() });
      if ('kind' in parsed) {
        if (parsed.kind === 'submit_with_submission' && parsed.submission) {
          const match = /^(\/\S+)(?:\s+(.*))?$/.exec(value);
          const args = match?.[2] ? match[2].split(/\s+/) : [];
          await dispatchSubmission(parsed.submission, args);
        } else if (parsed.kind === 'reject') {
          pushToast({ kind: 'error', message: parsed.message ?? t('composer.commandRejected') });
        } else if (parsed.kind === 'no-op') {
          pushToast({ kind: 'info', message: parsed.message ?? t('composer.commandHandled') });
        }
      } else {
        if (attachments.attachments.length > 0) {
          await useAgentStore.getState().submitItems([
            ...(value ? [{ type: 'text' as const, text: value }] : []),
            ...attachments.toUserInputItems(),
          ]);
        } else {
          await submit(value);
        }
        history.commit(value);
      }
      attachments.clear();
    } catch (error) {
      console.error(error);
      pushToast({ kind: 'error', message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusy(false);
      focus();
    }
  }, [
    activeId,
    attachments,
    dispatchSubmission,
    focus,
    history,
    pushToast,
    setBusy,
    setSlashVisible,
    setText,
    submit,
    t,
    text,
  ]);
}
