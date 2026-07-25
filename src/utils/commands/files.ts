/**
 * File tree + read_file wrappers — view/edit the active workspace.
 */
import { invoke } from '../bridge';

export interface ReflectDirEntry {
  name: string;
  path: string;
  /** "file" | "dir" | "symlink". */
  kind: string;
  size: number;
  /** Unix mtime seconds. */
  mtime: number;
  /** Depth from root (0 = root contents). */
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
 * List a directory. Skips dotfiles + common heavy dirs.
 * @param path  absolute path, or `null` for the active workspace
 * @param maxDepth  defaults to 4; caps entries at 2000.
 */
export async function reflect_list_dir(
  path: string | null,
  maxDepth = 4,
): Promise<ReflectDirListing> {
  return invoke<ReflectDirListing>('reflect_list_dir', { path, maxDepth: maxDepth });
}

/** Read a text file (max 1 MiB). Marks `binary=true` and returns empty content for binary. */
export async function reflect_read_file(path: string): Promise<ReflectFileReadResult> {
  return invoke<ReflectFileReadResult>('reflect_read_file', { path });
}
