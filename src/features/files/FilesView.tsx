/**
 * M3.x Files —— 工作区文件浏览器。
 *
 * - 列出当前 workspace 根目录文件
 * - 点击进入子目录
 * - M3.x 扩展:文件内容预览 / 编辑 / 搜索
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number;
}

export function FilesView() {
  const [cwd, setCwd] = useState<string>('.');
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      // M3.x: replace with real Rust command `reflect_list_files`
      // For now, try Tauri invoke, fallback to empty
      const result: FileEntry[] = await invoke('reflect_list_files', { cwd });
      setEntries(result);
    } catch {
      setEntries([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [cwd]);

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Files</h1>
      <div style={{ marginBottom: 12, fontSize: 12, color: '#666' }}>
        cwd: <code>{cwd}</code>
        {cwd !== '.' && (
          <button onClick={() => setCwd('.')} style={{ marginLeft: 8, cursor: 'pointer' }}>
            ↑ up
          </button>
        )}
      </div>
      {loading && <p style={{ color: '#888' }}>Loading…</p>}
      {error && <p style={{ color: 'crimson' }}>{error}</p>}
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
        <thead>
          <tr style={{ borderBottom: '1px solid #e2e8f0' }}>
            <th style={{ textAlign: 'left', padding: 6 }}>Name</th>
            <th style={{ textAlign: 'right', padding: 6 }}>Size</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((e) => (
            <tr
              key={e.path}
              style={{ borderBottom: '1px solid #f1f5f9', cursor: 'pointer' }}
              onClick={() => e.is_dir && setCwd(e.path)}
            >
              <td style={{ padding: 6 }}>
                {e.is_dir ? '📁' : '📄'} {e.name}
              </td>
              <td style={{ textAlign: 'right', padding: 6, color: '#888' }}>
                {e.size !== undefined ? `${e.size} B` : ''}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
