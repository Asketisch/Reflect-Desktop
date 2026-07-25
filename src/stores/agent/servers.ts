import type { LspServerEntry, McpServerEntry } from './types';

export function upsertServer<T extends McpServerEntry | LspServerEntry>(
  list: T[],
  name: string,
  status: 'started' | 'failed',
  detail?: string,
): T[] {
  const index = list.findIndex((server) => server.name === name);
  if (index === -1) return [...list, { name, status, detail } as T];
  return list.map((server, i) =>
    i === index ? { ...server, status, detail } : server,
  );
}
