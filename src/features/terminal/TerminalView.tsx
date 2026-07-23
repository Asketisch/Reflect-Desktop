/**
 * Terminal —— 真实 shell exec panel (B8-01).
 *
 * 通过 `reflect_run_shell` 启动命令,按行流式输出 stdout/stderr,
 * 支持 kill、history 自动滚动、ANSI-safe 显示 (基础 SGR 颜色 → CSS class)。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Terminal as TerminalIcon, CornerDownLeft, Square, Trash2 } from 'lucide-react';
import { Icon } from '@/features/design-system';
import {
  reflect_kill_shell,
  reflect_run_shell,
  onTerminalOutput,
  type ReflectShellOutputChunk,
  type ReflectShellSession,
} from '@/utils/commands';
import s from './TerminalView.module.css';

interface TerminalLine {
  /** Monotonic sequence per session; rendered as a stable key. */
  seq: number;
  stream: 'stdout' | 'stderr' | 'exit' | 'error' | 'info' | 'input';
  text: string;
}

interface TerminalSession {
  id: string;
  command: string;
  cwd: string;
  status: 'running' | 'exited' | 'killed' | 'errored';
  exitCode?: number;
}

const MAX_LINES_PER_SESSION = 5_000;

export function TerminalView() {
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [lines, setLines] = useState<Record<string, TerminalLine[]>>({});
  const [activeId, setActiveId] = useState<string | null>(null);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const linesRef = useRef<Record<string, TerminalLine[]>>({});

  // keep ref in sync so the event handler can append without re-binding.
  useEffect(() => {
    linesRef.current = lines;
  }, [lines]);

  // subscribe to output events.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onTerminalOutput((chunk: ReflectShellOutputChunk) => {
      const next: TerminalLine = {
        seq: chunk.seq,
        stream:
          chunk.stream === 'stdout' || chunk.stream === 'stderr' || chunk.stream === 'exit' || chunk.stream === 'error'
            ? chunk.stream
            : 'info',
        text: chunk.data,
      };
      const current = linesRef.current[chunk.session_id] ?? [];
      const merged = [...current, next];
      const trimmed = merged.length > MAX_LINES_PER_SESSION
        ? merged.slice(merged.length - MAX_LINES_PER_SESSION)
        : merged;
      linesRef.current = { ...linesRef.current, [chunk.session_id]: trimmed };
      setLines(linesRef.current);

      if (chunk.stream === 'exit') {
        setSessions((prev) =>
          prev.map((sess) =>
            sess.id === chunk.session_id
              ? { ...sess, status: 'exited', exitCode: Number(chunk.data) || 0 }
              : sess,
          ),
        );
      } else if (chunk.stream === 'error') {
        setSessions((prev) =>
          prev.map((sess) => (sess.id === chunk.session_id ? { ...sess, status: 'errored' } : sess)),
        );
      }
    }).then((un) => {
      unlisten = un;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // autoscroll on new lines for the active session.
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
  }, [lines, activeId]);

  const activeLines = activeId ? lines[activeId] ?? [] : [];
  const activeSession = useMemo(() => sessions.find((sess) => sess.id === activeId) ?? null, [sessions, activeId]);

  const run = useCallback(async () => {
    const cmd = input.trim();
    if (!cmd) return;
    setBusy(true);
    setInput('');
    try {
      const sess: ReflectShellSession = await reflect_run_shell(cmd);
      setSessions((prev) => [
        ...prev,
        { id: sess.id, command: sess.command, cwd: sess.cwd, status: 'running' },
      ]);
      setLines((prev) => ({
        ...prev,
        [sess.id]: [
          { seq: 0, stream: 'input', text: `$ ${cmd}` },
          { seq: 1, stream: 'info', text: `cwd: ${sess.cwd}` },
        ],
      }));
      setActiveId(sess.id);
    } catch (e) {
      // Surface as a synthetic error session so the user sees the failure.
      const errId = `err-${Date.now()}`;
      setSessions((prev) => [
        ...prev,
        { id: errId, command: cmd, cwd: '', status: 'errored' },
      ]);
      setLines((prev) => ({
        ...prev,
        [errId]: [{ seq: 0, stream: 'error', text: e instanceof Error ? e.message : String(e) }],
      }));
      setActiveId(errId);
    } finally {
      setBusy(false);
    }
  }, [input]);

  const killActive = useCallback(async () => {
    if (!activeId) return;
    const sess = sessions.find((s) => s.id === activeId);
    if (!sess || sess.status !== 'running') return;
    await reflect_kill_shell(activeId);
    setSessions((prev) =>
      prev.map((s) => (s.id === activeId ? { ...s, status: 'killed' } : s)),
    );
  }, [activeId, sessions]);

  const clearActive = useCallback(() => {
    if (!activeId) return;
    setLines((prev) => ({ ...prev, [activeId]: [] }));
  }, [activeId]);

  return (
    <div className={s.root} data-testid="terminal-root">
      <div className={s.header}>
        <Icon icon={TerminalIcon} size={14} />
        <span>Terminal</span>
        <span className={s.spacer} />
        {sessions.length > 0 && (
          <span className={s.sessionCount}>
            {sessions.length} session{sessions.length === 1 ? '' : 's'}
          </span>
        )}
      </div>

      {sessions.length > 0 && (
        <div className={s.tabs} role="tablist" aria-label="Shell sessions">
          {sessions.map((sess) => (
            <button
              key={sess.id}
              role="tab"
              type="button"
              aria-selected={sess.id === activeId}
              className={s.tab}
              data-active={sess.id === activeId}
              data-status={sess.status}
              onClick={() => setActiveId(sess.id)}
            >
              <span className={s.tabTitle}>
                {sess.command.length > 22 ? sess.command.slice(0, 22) + '…' : sess.command}
              </span>
              <span className={s.tabStatus}>{sess.status}</span>
            </button>
          ))}
        </div>
      )}

      <div className={s.screen} role="log" aria-label="Terminal output" aria-live="polite">
        {sessions.length === 0 ? (
          <div className={s.emptyWrap}>
            <p className={s.emptyTitle}>No active sessions</p>
            <p className={s.emptyHint}>Type a shell command below and press Enter.</p>
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
            onClick={killActive}
            data-testid="terminal-kill"
            aria-label="Kill running session"
          >
            <Icon icon={Square} size={12} /> Kill
          </button>
        )}
        {activeSession && (
          <button
            type="button"
            className={s.clearBtn}
            onClick={clearActive}
            data-testid="terminal-clear"
            aria-label="Clear session output"
          >
            <Icon icon={Trash2} size={12} /> Clear
          </button>
        )}
        {activeSession && (
          <span className={s.sessionMeta} data-testid="terminal-meta">
            {activeSession.status}
            {activeSession.exitCode !== undefined ? ` · exit ${activeSession.exitCode}` : ''}
            {' · cwd '}
            {activeSession.cwd}
          </span>
        )}
      </div>

      <form
        className={s.form}
        onSubmit={(e) => {
          e.preventDefault();
          if (!busy) void run();
        }}
      >
        <span className={s.prompt}>$</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          className={s.input}
          placeholder="echo hello"
          autoFocus
          disabled={busy}
          data-testid="terminal-input"
        />
        <Icon icon={CornerDownLeft} size={12} className={s.enterHint} />
      </form>
    </div>
  );
}