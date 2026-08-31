/**
 * useBangShell —— Composer `!` 终端直通（P3，对标 Reasonix「! 终端」语法）。
 *
 * `! <cmd>` 不进 agent 循环：经 `reflect_run_shell` 在当前工作区本地执行，
 * 输出经共享的 `reflect_terminal_output` 事件订阅按 session_id 分发到对应
 * run，`exit` / `error` 块收尾。结果只在 Composer 上方的面板本地呈现
 * （不写入会话 JSONL —— 这是用户自查环境，不是给模型的上下文）。
 *
 * 共享订阅：模块级一次性 `onTerminalOutput`，`Map<session_id, handler>`
 * 分发 —— 多个 Composer 实例（理论上单实例）与终端视图互不干扰。
 */
import { useCallback, useState } from 'react';
import {
  onTerminalOutput,
  reflect_kill_shell,
  reflect_run_shell,
  type ReflectShellOutputChunk,
} from '@/utils/commands';

export interface BangRun {
  /** shell session id（与后端 run_shell 返回一致）。 */
  id: string;
  command: string;
  output: string;
  status: 'running' | 'done' | 'error';
}

let sharedListenerStarted = false;
const outputTargets = new Map<string, (chunk: ReflectShellOutputChunk) => void>();

async function ensureSharedListener(): Promise<void> {
  if (sharedListenerStarted) return;
  try {
    await onTerminalOutput((chunk) => {
      outputTargets.get(chunk.session_id)?.(chunk);
    });
    sharedListenerStarted = true;
  } catch {
    // 注册失败保持 false:下一次 run 重试,而不是此后所有 run 永久收不到输出。
  }
}

const MAX_KEEP = 5;

export function useBangShell() {
  const [runs, setRuns] = useState<BangRun[]>([]);

  const patch = useCallback((id: string, fn: (run: BangRun) => BangRun) => {
    setRuns((prev) => prev.map((run) => (run.id === id ? fn(run) : run)));
  }, []);

  const run = useCallback(
    async (command: string) => {
      const trimmed = command.trim();
      if (!trimmed) return;
      await ensureSharedListener();
      try {
        const session = await reflect_run_shell(trimmed);
        outputTargets.set(session.id, (chunk) => {
          if (chunk.stream === 'exit' || chunk.stream === 'error') {
            outputTargets.delete(session.id);
            // 后端约定:`exit` 流的 data 是退出码字符串(`"0"`/`"1"`/空),
            // 非零退出码视为 error;`error` 流(spawn/wait 失败)直接标记 error。
            const trimmed = chunk.data.trim();
            const isError =
              chunk.stream === 'error' ||
              (chunk.stream === 'exit' && trimmed !== '' && trimmed !== '0');
            patch(session.id, (r) => ({
              ...r,
              output: chunk.data ? `${r.output}${chunk.data}` : r.output,
              status: isError ? 'error' : 'done',
            }));
            return;
          }
          patch(session.id, (r) => ({ ...r, output: `${r.output}${chunk.data}` }));
        });
        setRuns((prev) => [
          { id: session.id, command: trimmed, output: '', status: 'running' as const },
          ...prev.slice(0, MAX_KEEP - 1),
        ]);
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        setRuns((prev) => [
          { id: `local-${Date.now()}`, command: trimmed, output: message, status: 'error' },
          ...prev.slice(0, MAX_KEEP - 1),
        ]);
      }
    },
    [patch],
  );

  const dismiss = useCallback((id: string) => {
    outputTargets.delete(id);
    setRuns((prev) => prev.filter((r) => r.id !== id));
  }, []);

  const kill = useCallback(
    async (id: string) => {
      if (!id.startsWith('local-')) {
        await reflect_kill_shell(id).catch(() => {/* 已退出的 run 幂等 no-op */});
      }
      dismiss(id);
    },
    [dismiss],
  );

  return { runs, run, dismiss, kill };
}
