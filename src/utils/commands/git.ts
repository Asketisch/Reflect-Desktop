/**
 * Git 集成封装 —— status/diff/log。
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

/** 获取当前仓库状态(branch、upstream、entries、原始 porcelain 输出)。 */
export async function reflect_git_status(): Promise<ReflectGitStatus> {
  return invoke<ReflectGitStatus>('reflect_git_status');
}

/** 获取 diff 文本(`staged=false` 为工作区,`true` 为暂存区)。 */
export async function reflect_git_diff(staged = false): Promise<string> {
  return invoke<string>('reflect_git_diff', { staged });
}

/** 获取最近 N 条 log(默认 20)。 */
export async function reflect_git_log(limit = 20): Promise<ReflectGitLogEntry[]> {
  return invoke<ReflectGitLogEntry[]>('reflect_git_log', { limit });
}
