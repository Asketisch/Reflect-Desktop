/**
 * Find-in-files 封装 —— 在工作区内做 grep 搜索。
 */
import { invoke } from '../bridge';

export interface ReflectFileSearchHit {
  path: string;
  line: number;
  context: string;
}

export interface ReflectFileSearchResult {
  root: string;
  query: string;
  hits: ReflectFileSearchHit[];
  truncated: boolean;
}

/**
 * 在工作区内做子串搜索。
 *
 * @param query  待搜索文本
 * @param path   根路径,或 `null` 表示当前工作区
 * @param maxResults  结果上限(默认 200)
 */
export async function reflect_search_files(
  query: string,
  path: string | null = null,
  maxResults = 200,
): Promise<ReflectFileSearchResult | null> {
  return invoke<ReflectFileSearchResult | null>('reflect_search_files', {
    query,
    path,
    maxResults,
  });
}
