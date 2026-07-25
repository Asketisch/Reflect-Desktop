/**
 * Terminal / shell exec wrappers — spawn/kill/list + output subscription.
 *
 * Output chunks stream over the `reflect_terminal_output` event. The front-end
 * subscribes once per session via `onTerminalOutput` and filters by `session_id`.
 */
import { invoke, listen } from '../bridge';

export interface ReflectShellSession {
  id: string;
  command: string;
  cwd: string;
}

export interface ReflectShellOutputChunk {
  session_id: string;
  /** "stdout" | "stderr" | "exit" | "error". */
  stream: string;
  data: string;
  seq: number;
}

/** Spawn a shell command; output streams as `reflect_terminal_output` events. */
export async function reflect_run_shell(cmd: string): Promise<ReflectShellSession> {
  return invoke<ReflectShellSession>('reflect_run_shell', { cmd });
}

/** Kill a running shell session by id. Idempotent. */
export async function reflect_kill_shell(session_id: string): Promise<void> {
  return invoke<void>('reflect_kill_shell', { session_id });
}

/** Diagnostic: list active shell session ids. */
export async function reflect_list_shell_sessions(): Promise<string[]> {
  return invoke<string[]>('reflect_list_shell_sessions');
}

/** Subscribe to terminal output events for any session. */
export async function onTerminalOutput(handler: (chunk: ReflectShellOutputChunk) => void) {
  return listen<ReflectShellOutputChunk>('reflect_terminal_output', (e) => handler(e.payload));
}
