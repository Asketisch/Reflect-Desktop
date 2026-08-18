/**
 * Terminal —— 真实 shell 执行面板（B8-01）。
 *
 * 通过 `reflect_run_shell` 启动命令，按行流式输出 stdout/stderr，
 * 支持 kill、history 自动滚动、ANSI-safe 显示（基础 SGR 颜色 → CSS class）。
 *
 * 2026-07-25 重构：状态 + 事件编排位于 `useTerminalController`；
 * 此组件为纯展示组件。
 */
import { useEffect, useRef } from 'react';
import { Terminal as TerminalIcon, CornerDownLeft, Square, Trash2 } from 'lucide-react';
import { Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { useTerminalController } from './useTerminalController';
import s from './TerminalView.module.css';

export function TerminalView() {
  const { t, tp } = useI18n();
  const ctrl = useTerminalController();
  const { state, activeLines, activeSession } = ctrl;
  const bottomRef = useRef<HTMLDivElement>(null);

  // 活动 session 的新行自动滚动。
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
  }, [activeLines, state.activeId]);

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!state.busy) void ctrl.run(state.input);
  };

  return (
    <div className={s.root} data-testid="terminal-root">
      <div className={s.header}>
        <Icon icon={TerminalIcon} size={14} />
        <span>{t('terminal.title')}</span>
        <span className={s.spacer} />
        {state.sessions.length > 0 && (
          <span className={s.sessionCount}>
            {tp('terminal.sessionCount', state.sessions.length, { count: state.sessions.length })}
          </span>
        )}
      </div>

      {state.sessions.length > 0 && (
        <div className={s.tabs} role="tablist" aria-label={t('terminal.sessionsAria')}>
          {state.sessions.map((sess) => (
            <button
              key={sess.id}
              role="tab"
              type="button"
              aria-selected={sess.id === state.activeId}
              className={s.tab}
              data-active={sess.id === state.activeId}
              data-status={sess.status}
              onClick={() => ctrl.selectSession(sess.id)}
            >
              <span className={s.tabTitle}>
                {sess.command.length > 22 ? sess.command.slice(0, 22) + '…' : sess.command}
              </span>
              <span className={s.tabStatus}>{sess.status}</span>
            </button>
          ))}
        </div>
      )}

      <div className={s.screen} role="log" aria-label={t('terminal.outputAria')} aria-live="polite">
        {state.sessions.length === 0 ? (
          <div className={s.emptyWrap}>
            <p className={s.emptyTitle}>{t('terminal.empty.title')}</p>
            <p className={s.emptyHint}>{t('terminal.empty.hint')}</p>
          </div>
        ) : (
          activeLines.map((line) => (
            <div key={line.seq} className={s.line} data-stream={line.stream}>
              {line.text}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>

      <div className={s.toolbar}>
        {activeSession && activeSession.status === 'running' && (
          <button
            type="button"
            className={s.killBtn}
            onClick={() => void ctrl.killActive()}
            data-testid="terminal-kill"
            aria-label={t('terminal.kill')}
          >
            <Icon icon={Square} size={12} /> {t('terminal.kill')}
          </button>
        )}
        {activeSession && (
          <button
            type="button"
            className={s.clearBtn}
            onClick={ctrl.clearActive}
            data-testid="terminal-clear"
            aria-label={t('terminal.clear')}
          >
            <Icon icon={Trash2} size={12} /> {t('terminal.clear')}
          </button>
        )}
        {activeSession && (
          <span className={s.sessionMeta} data-testid="terminal-meta">
            {activeSession.status}
            {activeSession.exitCode !== undefined ? ` · ${t('terminal.exit')} ${activeSession.exitCode}` : ''}
            {' · '}{t('terminal.cwd')} {activeSession.cwd}
          </span>
        )}
      </div>

      <form className={s.form} onSubmit={onSubmit}>
        <span className={s.prompt}>$</span>
        <input
          value={state.input}
          onChange={(e) => ctrl.setInput(e.target.value)}
          className={s.input}
          placeholder="echo hello"
          autoFocus
          disabled={state.busy}
          data-testid="terminal-input"
        />
        <Icon icon={CornerDownLeft} size={12} className={s.enterHint} />
      </form>
    </div>
  );
}
