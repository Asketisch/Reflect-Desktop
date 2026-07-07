/**
 * M3.x Update —— 更新检查。
 *
 * - 检查 Reflect Desktop 更新
 * - 显示当前版本 + 最新版本
 * - 下载 / 安装更新
 * - M3.x 扩展:auto-update + changelog
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  changelog?: string;
}

export function UpdateView() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [downloading, setDownloading] = useState(false);

  const check = async () => {
    setLoading(true);
    try {
      const result: UpdateInfo = await invoke('reflect_check_update');
      setInfo(result);
    } catch {
      setInfo({ current: '0.1.0', latest: '0.1.0', available: false });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { check(); }, []);

  const onDownload = async () => {
    setDownloading(true);
    try {
      await invoke('reflect_download_update');
    } catch { /* ignore */ }
    setDownloading(false);
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Updates</h1>

      <div style={{ padding: 16, border: '1px solid #e2e8f0', borderRadius: 8, marginBottom: 16 }}>
        <p style={{ margin: '4px 0', fontSize: 14 }}>
          <strong>Current:</strong> {info?.current ?? '0.1.0'}
        </p>
        <p style={{ margin: '4px 0', fontSize: 14 }}>
          <strong>Latest:</strong> {loading ? 'Checking…' : (info?.latest ?? '0.1.0')}
        </p>
        {info?.available && (
          <p style={{ margin: '8px 0 0', fontSize: 13, color: '#3b82f6' }}>
            Update available!
          </p>
        )}
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <button
          onClick={check}
          disabled={loading}
          style={{
            padding: '8px 16px',
            border: '1px solid #e2e8f0',
            background: 'white',
            borderRadius: 6,
            cursor: 'pointer',
            fontSize: 13,
          }}
        >
          Check for updates
        </button>
        {info?.available && (
          <button
            onClick={onDownload}
            disabled={downloading}
            style={{
              padding: '8px 16px',
              background: '#3b82f6',
              color: 'white',
              border: 'none',
              borderRadius: 6,
              cursor: 'pointer',
              fontSize: 13,
            }}
          >
            {downloading ? 'Downloading…' : 'Download'}
          </button>
        )}
      </div>

      {info?.changelog && (
        <section style={{ marginTop: 24 }}>
          <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>Changelog</h2>
          <pre style={{ background: '#f8fafc', padding: 12, borderRadius: 6, fontSize: 12, whiteSpace: 'pre-wrap' }}>
            {info.changelog}
          </pre>
        </section>
      )}
    </div>
  );
}
