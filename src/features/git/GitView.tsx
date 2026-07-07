/**
 * M3.x Git —— Git 状态 + 工作区变更。
 *
 * - 显示当前分支、状态 (clean / dirty)
 * - 列出 modified / untracked 文件
 * - M3.x 扩展:diff viewer + commit + push
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface GitStatus {
  branch: string;
  clean: boolean;
  files: Array<{ path: string; status: 'M' | 'A' | 'D' | '?' }>;
}

export function GitView() {
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      // M3.x: replace with real Rust command `reflect_git_status`
      const result: GitStatus = await invoke('reflect_git_status');
      setStatus(result);
    } catch {
      setStatus(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Git</h1>

      {loading && <p style={{ color: '#888' }}>Loading…</p>}
      {error && <p style={{ color: 'crimson' }}>{error}</p>}

      {status && (
        <>
          <div style={{ marginBottom: 16, fontSize: 13 }}>
            <strong>Branch:</strong> <code>{status.branch}</code>
            <span style={{ marginLeft: 12, color: status.clean ? '#22c55e' : '#f59e0b' }}>
              {status.clean ? 'clean' : `${status.files.length} changes`}
            </span>
          </div>

          {status.files.length > 0 && (
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
              <thead>
                <tr style={{ borderBottom: '1px solid #e2e8f0' }}>
                  <th style={{ textAlign: 'left', padding: 6, width: 60 }}>Status</th>
                  <th style={{ textAlign: 'left', padding: 6 }}>Path</th>
                </tr>
              </thead>
              <tbody>
                {status.files.map((f, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid #f1f5f9' }}>
                    <td style={{ padding: 6, color: statusColor(f.status) }}>{f.status}</td>
                    <td style={{ padding: 6 }}><code>{f.path}</code></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </div>
  );
}

function statusColor(s: string): string {
  switch (s) {
    case 'M': return '#3b82f6';
    case 'A': return '#22c55e';
    case 'D': return '#ef4444';
    case '?': return '#f59e0b';
    default: return '#888';
  }
}
