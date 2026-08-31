import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { dispatch } from './slashEngine';
import { dispatchSubmission } from './dispatchSubmission';
import type { UseAttachmentsResult } from './useAttachments';
import { useAgent } from '@/services/agent';
import {
  useSessions,
  useActiveSession,
  SESSIONS_QUERY_KEY,
} from '@/features/sessions/hooks/useSessions';
import { useI18n } from '@/utils/i18n';
import { reflect_interrupt } from '@/utils/commands';
import {
  useAgentStore,
  selectIsTurnRunning,
} from '@/stores/agentStore';
import type { UserInputItem } from '@/types/protocol';
import type { PromptHistoryApi } from './usePromptHistory';

/** C：运行中发送的两种语义 —— 排队（默认）或转向（中断当前 run 立即发送）。 */
export type SendMode = 'queue' | 'steer';

interface UseComposerSubmissionOptions {
  text: string;
  attachments: UseAttachmentsResult;
  history: PromptHistoryApi;
  setText: (text: string) => void;
  setBusy: (busy: boolean) => void;
  setSlashVisible: (visible: boolean) => void;
  focus: () => void;
  /** v1.x：当前工作区绝对路径；提交时注入 `ReflectSubmission.workspace`。 */
  currentWorkspace?: string | null;
  /** C：Queue/Steer 模式（仅 turn 运行中发送时生效）。 */
  sendMode?: SendMode;
  /** P3：`!` 终端直通 —— 文本以 `!` 开头时本地执行，不进 agent 循环。 */
  onBangCommand?: (command: string) => Promise<void>;
}

/**
 * steer：中断当前 turn，等它真正收尾（turn_aborted → 无 streaming turn）
 * 后再提交。轮询 store 而非订阅事件，避免一次性订阅的清理复杂度；
 * 超时兜底直接提交 —— 后端 mpsc 保序，中断先于新消息被处理。
 */
async function waitTurnSettled(timeoutMs = 8000): Promise<void> {
  const start = Date.now();
  while (selectIsTurnRunning(useAgentStore.getState())) {
    if (Date.now() - start > timeoutMs) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

export function useComposerSubmission({
  text,
  attachments,
  history,
  setText,
  setBusy,
  setSlashVisible,
  focus,
  currentWorkspace,
  sendMode = 'queue',
  onBangCommand,
}: UseComposerSubmissionOptions) {
  const { submit } = useAgent();
  const { activeId } = useActiveSession();
  const { rename } = useSessions();
  const pushToast = useAgentStore((state) => state.pushToast);
  const { t } = useI18n();
  const qc = useQueryClient();

  // slash submission 分发已抽到 dispatchSubmission.ts（Composer 与命令面板共用）。
  const runSubmission = useCallback(
    (kind: string, args: string[]) =>
      dispatchSubmission(kind, args, { activeSessionId: activeId, pushToast, t, rename }),
    [activeId, pushToast, rename, t],
  );

  return useCallback(async (options?: { reverseMode?: boolean }) => {
    const value = text.trim();
    if (!value && attachments.attachments.length === 0) return;

    // 单次反转：Shift+Cmd/Ctrl+Enter 时本次发送用对侧模式。
    const mode: SendMode = options?.reverseMode
      ? sendMode === 'queue'
        ? 'steer'
        : 'queue'
      : sendMode;

    setBusy(true);
    setText('');
    setSlashVisible(false);
    history.resetNavigation();
    try {
      // P3：`!` 终端直通 —— 本地执行，不进 agent 循环、不写会话历史；
      // 附件保留在输入框（用户可能接着补充说明再发送）。
      if (value.startsWith('!') && onBangCommand) {
        const command = value.slice(1).trim();
        if (command) {
          history.commit(value);
          await onBangCommand(command);
        } else {
          setText('!');
        }
        return;
      }
      const parsed = dispatch(value, { activeSessionId: activeId, now: () => new Date() });
      if ('kind' in parsed) {
        if (parsed.kind === 'submit_with_submission' && parsed.submission) {
          const match = /^(\/\S+)(?:\s+(.*))?$/.exec(value);
          const args = match?.[2] ? match[2].split(/\s+/) : [];
          await runSubmission(parsed.submission, args);
        } else if (parsed.kind === 'reject') {
          pushToast({ kind: 'error', message: parsed.message ?? t('composer.commandRejected') });
        } else if (parsed.kind === 'no-op') {
          pushToast({ kind: 'info', message: parsed.message ?? t('composer.commandHandled') });
        }
      } else {
        const items: UserInputItem[] = [
          ...(value ? [{ type: 'text' as const, text: value }] : []),
          ...attachments.toUserInputItems(),
        ];
        const running = selectIsTurnRunning(useAgentStore.getState());
        if (running && mode === 'steer') {
          // Steer：中断当前 run，收尾后立即发送；已完成工具调用保留在
          // 会话历史中，模型带着进度从新指令继续。
          await reflect_interrupt();
          await waitTurnSettled();
          // 收尾等待最长 8s,期间用户可能已切走会话(rebind 已换线程)。
          // 用实时路由比对,消息只能进发起时所在的会话。
          const routeSession = /^\/chat\/([^/]+)$/.exec(window.location.pathname)?.[1] ?? null;
          const currentRouteSession = routeSession ? decodeURIComponent(routeSession) : null;
          if (currentRouteSession !== activeId) {
            // 消息不再替用户发出去；保留失败提示（输入已被清空至少有据可查）。
            pushToast({ kind: 'warn', message: t('composer.steerCancelled') });
            return;
          }
          await useAgentStore.getState().submitItems(items, currentWorkspace ?? null);
          history.commit(value);
          void qc.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
        } else if (running) {
          // Queue：留在前端队列（不发后端），turn 收尾后自动排空；
          // 队列消息在消息流底部可见、可编辑、可撤销。
          useAgentStore.getState().enqueueMessage(items, currentWorkspace ?? null);
        } else if (attachments.attachments.length > 0) {
          await useAgentStore.getState().submitItems(items, currentWorkspace ?? null);
          history.commit(value);
          void qc.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
        } else {
          await submit(value, currentWorkspace ?? null);
          history.commit(value);
          // 刷新侧边栏会话列表:新会话的首条消息会派生标题,
          // 不等下一次手动刷新/缓存过期即可见。
          void qc.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
        }
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
    currentWorkspace,
    focus,
    history,
    onBangCommand,
    pushToast,
    qc,
    runSubmission,
    sendMode,
    setBusy,
    setSlashVisible,
    setText,
    submit,
    t,
    text,
  ]);
}
