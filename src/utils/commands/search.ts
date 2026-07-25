/**
 * Find-in-files wrappers — grep the workspace.
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
 * Substring search across the workspace.
 *
 * @param query  text to search for
 * @param path   root path, or `null` for the active workspace
 * @param maxResults  caps results (defaults to 200)
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
