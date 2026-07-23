/**
 * Slash engine —— 解析 + 分发 `/command args...` 输入。
 *
 * 契约：
 *   - `parseSlash(input)` 把文本切成 `{ command, args, raw, isSlash }`。
 *     `isSlash = false` 表示非 slash（普通提交）,由 caller 走 user_input_text。
 *   - `executeSlash(parsed, ctx)` 执行命令,返回 `SlashResult`:
 *       `{ kind: 'submit' | 'submit_with_submission' | 'no-op' | 'reject', payload, message }`
 *     其中 `submit_with_submission` 表示 caller 应该用 raw submission 而不是 freeform text
 *     (例如 `/compact` → 触发 compact,而不是把 "/compact" 当文本)。
 *   - 命令 → action 是纯映射;不副作用(纯函数),方便测试。
 *
 * B4 涵盖 46 个命令的核心语义。Tier A 9 个有"实装"的执行,其余标记为
 * 'no-op' 但仍然 parse + dispatch(避免 silent failure)。
 */
import { SLASH_COMMANDS, type SlashCmd } from './slashCommands';

export interface ParsedSlash {
  command: string;
  args: string[];
  raw: string;
  isSlash: true;
}

export interface PlainText {
  raw: string;
  isSlash: false;
}

export type SlashInput = ParsedSlash | PlainText;

export type SlashResultKind =
  | 'submit' // 普通文本提交,freeform
  | 'submit_with_submission' // 触发了特定 Submission (compact/interrupt/...)
  | 'no-op' // 解析成功但尚未实装,告诉 UI "command handled"
  | 'reject'; // 非法输入

export interface SlashResult {
  kind: SlashResultKind;
  /** Optional Submission kind to invoke (matches `reflect_*` command). */
  submission?: string;
  /** Plain-text payload when `kind === 'submit'`. */
  payload?: string;
  /** User-facing message (status, error, etc). */
  message?: string;
}

export interface SlashContext {
  activeSessionId: string | null;
  now: () => Date;
}

/** Parse `/cmd args...` from freeform text. */
export function parseSlash(input: string): SlashInput {
  const trimmed = input.trim();
  if (!trimmed.startsWith('/')) return { raw: trimmed, isSlash: false };
  // Match `/<cmd>` followed by zero or more space-separated args.
  const m = trimmed.match(/^\/(\S+)(?:\s+(.*))?$/);
  if (!m) return { raw: trimmed, isSlash: false };
  const command = m[1].toLowerCase();
  const rest = (m[2] ?? '').trim();
  const args = rest ? rest.split(/\s+/) : [];
  return { command, args, raw: trimmed, isSlash: true };
}

/** Look up a SlashCmd by primary name or alias. Returns null if unknown. */
export function resolveCommand(name: string): SlashCmd | null {
  const n = name.toLowerCase();
  return (
    SLASH_COMMANDS.find((c) => c.name === n) ??
    SLASH_COMMANDS.find((c) => c.aliases?.includes(n)) ??
    null
  );
}

/** Execute a parsed slash and return the dispatch result. Pure (no side effects). */
export function executeSlash(parsed: ParsedSlash, _ctx: SlashContext): SlashResult {
  const cmd = resolveCommand(parsed.command);
  if (!cmd) {
    return { kind: 'reject', message: `Unknown command: /${parsed.command}` };
  }

  switch (cmd.name) {
    // ===== Tier A — 实装 =====
    case 'compact':
      return { kind: 'submit_with_submission', submission: 'compact', message: 'Compacting context…' };
    case 'plan':
      return {
        kind: 'submit_with_submission',
        submission: 'enter_plan_mode',
        message: `Entering plan mode (task: ${parsed.args.join(' ') || 'unspecified'})…`,
      };
    case 'exit-plan':
      return { kind: 'submit_with_submission', submission: 'exit_plan_mode', message: 'Exiting plan mode…' };
    case 'effort': {
      const level = parsed.args[0]?.toLowerCase();
      if (!level || !['low', 'medium', 'high'].includes(level)) {
        return { kind: 'reject', message: '/effort requires: low | medium | high' };
      }
      return {
        kind: 'submit_with_submission',
        submission: 'set_effort',
        message: `Effort → ${level}`,
      };
    }
    case 'mode': {
      const m = parsed.args[0]?.toLowerCase();
      if (!m || !['auto', 'prompt', 'deny', 'plan'].includes(m)) {
        return { kind: 'reject', message: '/mode requires: auto | prompt | deny | plan' };
      }
      return {
        kind: 'submit_with_submission',
        submission: 'set_permission_mode',
        message: `Permission mode → ${m}`,
      };
    }
    case 'model':
    case 'provider':
      // 后续在 ModelsView 处理;此处仅路由标记。
      return { kind: 'no-op', message: `Open Settings → Models to change ${cmd.name}.` };
    case 'theme':
      return { kind: 'no-op', message: 'Open Settings → Theme.' };
    case 'vim':
      return { kind: 'no-op', message: 'Toggle vim mode in Settings → Keymap.' };
    case 'status':
      return { kind: 'no-op', message: 'See StatusBar at the bottom of the window.' };
    case 'help':
      return { kind: 'no-op', message: `Slash commands: ${SLASH_COMMANDS.length} total. Type / to browse.` };

    // ===== Tier B — 部分实装 =====
    case 'clear':
      return { kind: 'no-op', message: 'New chat — navigate to /chat.' };
    case 'resume':
      return { kind: 'no-op', message: 'Open ThreadsView (sidebar) to resume a session.' };
    case 'rename': {
      const name = parsed.args.join(' ').trim();
      if (!_ctx.activeSessionId) {
        return { kind: 'reject', message: '/rename requires an active session.' };
      }
      if (!name) {
        return { kind: 'reject', message: '/rename <new name>' };
      }
      // 实装在 Composer caller 里(需要 useSessions.rename 注入)。
      return {
        kind: 'submit_with_submission',
        submission: 'rename_session',
        message: `Renaming → "${name}"`,
      };
    }
    case 'export':
      if (!_ctx.activeSessionId) {
        return { kind: 'reject', message: '/export requires an active session.' };
      }
      return { kind: 'submit_with_submission', submission: 'export_session', message: 'Exporting…' };
    case 'interrupt':
      return { kind: 'submit_with_submission', submission: 'interrupt', message: 'Interrupting…' };

    // ===== Tier C — stubs that route to settings / no-op =====
    case 'init':
    case 'diff':
    case 'files':
    case 'branch':
    case 'commit':
    case 'commit-push-pr':
    case 'review':
    case 'ultrareview':
    case 'cost':
    case 'usage':
    case 'copy':
    case 'editor':
    case 'stats':
    case 'insights':
    case 'context':
    case 'ctx_viz':
    case 'color':
    case 'sandbox-toggle':
    case 'memory':
    case 'hooks':
    case 'tasks':
    case 'permissions':
    case 'skills':
    case 'keybindings':
    case 'statusline':
    case 'output-style':
    case 'mcp':
    case 'plugin':
    case 'ide':
    case 'desktop':
    case 'mobile':
    case 'exit':
    case 'session':
    case 'share':
    default:
      return { kind: 'no-op', message: `/${cmd.name} — handled by Settings / dedicated view.` };
  }
}

/** Top-level dispatcher: parse + execute in one call. */
export function dispatch(input: string, ctx: SlashContext): SlashInput | SlashResult {
  const parsed = parseSlash(input);
  if (!parsed.isSlash) return parsed;
  return executeSlash(parsed, ctx);
}
