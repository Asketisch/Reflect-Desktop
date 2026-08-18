/**
 * Session I/O —— 列表 / 回放 / 重命名 / 删除 / 导出。
 *
 * `ReflectRolloutRecord` 故意采用 `Record<string, unknown>` 形态,
 * 因为 Rust 端返回的是按协议判别的联合类型;TS 视图见
 * `src/types/protocol.ts`。
 */
import { invoke } from '../bridge';
import type { ReflectRolloutRecord, ReflectSessionInfo } from '../types';

/** `reflect_list_sessions` 返回的 session 元数据。 */
export type { ReflectSessionInfo } from '../types';

/** `reflect_export_session_markdown` 的返回结果。 */
export interface ReflectMarkdownExportResult {
  path: string;
  bytes: number;
}

/** 分页获取会话列表。`limit` 与 `offset` 均可选。 */
export async function reflect_list_sessions(
  opts: { limit?: number; offset?: number } = {},
): Promise<ReflectSessionInfo[]> {
  return invoke<ReflectSessionInfo[]>('reflect_list_sessions', {
    limit: opts.limit ?? null,
    offset: opts.offset ?? null,
  });
}

/** 按 id 重命名已有会话。 */
export async function reflect_rename_session(id: string, new_name: string): Promise<void> {
  return invoke<void>('reflect_rename_session', { id, newName: new_name });
}

/** 按 id 删除已有会话。 */
export async function reflect_delete_session(id: string): Promise<void> {
  return invoke<void>('reflect_delete_session', { id });
}

/** 回放会话:返回原始事件所产生的 rollout record 流。 */
export async function reflect_replay_session(id: string): Promise<ReflectRolloutRecord[]> {
  return invoke<ReflectRolloutRecord[]>('reflect_replay_session', { id });
}

/** 将会话导出为 JSON,返回绝对路径。 */
export async function reflect_export_session(id: string): Promise<string | null> {
  return invoke<string | null>('reflect_export_session', { id });
}

/** 将会话导出为 Markdown,返回渲染后的文件路径。 */
export async function reflect_export_session_markdown(
  id: string,
): Promise<ReflectMarkdownExportResult | null> {
  return invoke<ReflectMarkdownExportResult | null>('reflect_export_session_markdown', { id });
}
