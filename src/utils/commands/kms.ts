/**
 * KMS (Knowledge Management System) IPC wrappers.
 *
 * Phase 3 item 12: grep-based wiki + /dream session mining.
 */
import { invoke } from '@/utils/bridge';

// ── Types (camelCase, matching Rust serde(rename_all = "camelCase")) ──

export interface ReflectWikiInfo {
  name: string;
  description: string | null;
  path: string;
  pageCount: number;
  modifiedAtMs: number;
}

export interface ReflectPage {
  name: string;
  wiki: string;
  content: string;
  title: string | null;
  tags: string[];
  modifiedAtMs: number;
}

export interface ReflectSearchResult {
  wiki: string;
  page: string;
  lines: string[];
  lineCount: number;
}

export interface ReflectDreamResult {
  insights: string[];
  sessionsAnalyzed: number;
  wiki: string;
  page: string;
}

// ── Wrappers ──

export function reflect_kms_list(): Promise<ReflectWikiInfo[]> {
  return invoke('reflect_kms_list');
}

export function reflect_kms_create(
  name: string,
  description: string | null,
): Promise<ReflectWikiInfo> {
  return invoke('reflect_kms_create', { name, description });
}

export function reflect_kms_delete(name: string): Promise<void> {
  return invoke('reflect_kms_delete', { name });
}

export function reflect_kms_save_page(
  wiki: string,
  page: string,
  content: string,
  title: string | null,
  tags: string[] | null,
): Promise<ReflectPage> {
  return invoke('reflect_kms_save_page', { wiki, page, content, title, tags });
}

export function reflect_kms_get_page(wiki: string, page: string): Promise<ReflectPage> {
  return invoke('reflect_kms_get_page', { wiki, page });
}

export function reflect_kms_list_pages(wiki: string): Promise<ReflectPage[]> {
  return invoke('reflect_kms_list_pages', { wiki });
}

export function reflect_kms_search(query: string): Promise<ReflectSearchResult[]> {
  return invoke('reflect_kms_search', { query });
}

export function reflect_dream(insights: string[], sessionsAnalyzed: number): Promise<ReflectDreamResult> {
  return invoke('reflect_dream', { insights, sessionsAnalyzed });
}