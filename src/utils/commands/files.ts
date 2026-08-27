/**
 * 文件树 + read_file 封装 —— 浏览/编辑当前工作区。
 */
import { invoke } from '../bridge';

export interface ReflectDirEntry {
  name: string;
  path: string;
  /** "file" | "dir" | "symlink"。 */
  kind: string;
  size: number;
  /** Unix mtime 秒数。 */
  mtime: number;
  /** 距根目录的深度(0 = 根目录内容)。 */
  depth: number;
}

export interface ReflectDirListing {
  root: string;
  entries: ReflectDirEntry[];
  truncated: boolean;
}

export interface ReflectFileReadResult {
  path: string;
  content: string;
  size: number;
  binary: boolean;
  truncated: boolean;
}

/**
 * 列出目录内容。跳过 dotfile 与常见的重型目录。
 * @param path  绝对路径,或 `null` 表示当前工作区
 * @param maxDepth  默认 4;总条目上限 2000。
 */
export async function reflect_list_dir(
  path: string | null,
  maxDepth = 4,
): Promise<ReflectDirListing> {
  return invoke<ReflectDirListing>('reflect_list_dir', { path, maxDepth: maxDepth });
}

/**
 * v1.x：Composer `@` 弹层专用 —— 列出当前工作区 depth=2 的目录条目。
 * 后端 `reflect_list_dir` 已跳过 dotfile + node_modules / target / dist / .git
 * （详见 `commands/files.rs::should_skip`），无需前端再做过滤。
 *
 * @param maxDepth  默认 2。深度 1 只列顶层目录；2 含一层子目录；过深会
 *                  触及 2000 条上限（monorepo 截断风险）。
 */
export async function listDirForMention(
  maxDepth = 2,
): Promise<ReflectDirListing> {
  return invoke<ReflectDirListing>('reflect_list_dir', {
    path: null,
    maxDepth,
  });
}

/** 读取文本文件(最大 1 MiB)。对二进制文件标记 `binary=true` 并返回空内容。 */
export async function reflect_read_file(path: string): Promise<ReflectFileReadResult> {
  return invoke<ReflectFileReadResult>('reflect_read_file', { path });
}
