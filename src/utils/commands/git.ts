/**
 * Git integration wrappers — status/diff/log.
 */
import { invoke } from '../bridge';

export interface ReflectGitStatusEntry {
  path: string;
  status: string;
  old_path: string | null;
}

export interface ReflectGitStatus {
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  entries: ReflectGitStatusEntry[];
  raw: string;
  is_repo: boolean;
}

export interface ReflectGitLogEntry {
  hash: string;
  short_hash: string;
  author: string;
  timestamp: number;
  subject: string;
}

/** Get the active repository status (branch, upstream, entries, raw porcelain). */
export async function reflect_git_status(): Promise<ReflectGitStatus> {
  return invoke<ReflectGitStatus>('reflect_git_status');
}

/** Get the diff text (`staged=false` for working tree, `true` for staged). */
export async function reflect_git_diff(staged = false): Promise<string> {
  return invoke<string>('reflect_git_diff', { staged });
}

/** Get the last N log entries (default 20). */
export async function reflect_git_log(limit = 20): Promise<ReflectGitLogEntry[]> {
  return invoke<ReflectGitLogEntry[]>('reflect_git_log', { limit });
}
