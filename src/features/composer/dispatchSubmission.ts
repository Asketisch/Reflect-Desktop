/**
 * slash submission 分发 —— `submit_with_submission` 类命令的唯一执行实现。
 *
 * 此前该 switch 只存在于 useComposerSubmission 内部,命令面板的 runSlash
 * 把 "/compact" 等字面文本 submit 给了模型(后端不解析 slash 文本)。
 * 抽到这里后 Composer 与命令面板共用同一条管线。
 */
import {
  reflect_compact,
  reflect_interrupt,
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  reflect_enter_goal_mode,
  reflect_exit_goal_mode,
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_export_session,
} from '@/utils/commands';
import type { LocaleKey } from '@/utils/i18n';

export type ToastKind = 'info' | 'success' | 'warn' | 'error';

export interface DispatchSubmissionCtx {
  activeSessionId: string | null;
  pushToast: (input: { kind: ToastKind; message: string }) => void;
  t: (key: LocaleKey, vars?: Record<string, string | number>) => string;
  /** useSessions().rename —— /rename 需要落到会话列表缓存。 */
  rename: (id: string, name: string) => Promise<void>;
}

/** 执行一个 slash submission kind。kind 来自 slashEngine.executeSlash 的结果。 */
export async function dispatchSubmission(
  kind: string,
  args: string[],
  ctx: DispatchSubmissionCtx,
): Promise<void> {
  const { activeSessionId, pushToast, t, rename } = ctx;
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
    case 'enter_goal_mode':
      await reflect_enter_goal_mode(args.join(' '));
      break;
    case 'exit_goal_mode':
      await reflect_exit_goal_mode();
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
      if (activeSessionId) {
        const path = await reflect_export_session(activeSessionId);
        pushToast({
          kind: 'info',
          message: path ? t('composer.exported', { path }) : t('composer.exportedNoPath'),
        });
      }
      break;
    case 'rename_session': {
      // /rename "My Name" 的引号在这里剥离,避免会话名带上字面引号。
      const name = args
        .join(' ')
        .trim()
        .replace(/^"(.*)"$/, '$1')
        .replace(/^'(.*)'$/, '$1')
        .trim();
      if (activeSessionId && name) await rename(activeSessionId, name);
      break;
    }
    default:
      pushToast({ kind: 'warn', message: t('composer.unhandledSlash', { kind }) });
  }
}
