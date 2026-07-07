/**
 * M3.x About —— 关于 Reflect Desktop。
 *
 * - 版本号、构建信息
 * - 依赖开源项目致谢
 * - 系统信息 (OS、架构、Tauri 版本)
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SystemInfo {
  os: string;
  arch: string;
  tauriVersion: string;
}

export function AboutView() {
  const [sys, setSys] = useState<SystemInfo | null>(null);

  useEffect(() => {
    invoke<SystemInfo>('reflect_system_info').then(setSys).catch(() => setSys(null));
  }, []);

  return (
    <div style={{ padding: 32, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>About Reflect Desktop</h1>

      <div style={{ marginBottom: 24 }}>
        <p style={{ margin: '4px 0', fontSize: 14 }}>
          <strong>Version:</strong> 0.1.0
        </p>
        <p style={{ margin: '4px 0', fontSize: 14 }}>
          <strong>Build:</strong> {new Date().toLocaleDateString()}
        </p>
        {sys && (
          <>
            <p style={{ margin: '4px 0', fontSize: 14 }}>
              <strong>OS:</strong> {sys.os} ({sys.arch})
            </p>
            <p style={{ margin: '4px 0', fontSize: 14 }}>
              <strong>Tauri:</strong> {sys.tauriVersion}
            </p>
          </>
        )}
      </div>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 16, marginBottom: 8 }}>Reflect Agent</h2>
        <p style={{ fontSize: 13, color: '#666' }}>
          Standalone desktop GUI for Reflect Agent — an AI coding agent that helps you write,
          review, and refactor code.
        </p>
      </section>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 16, marginBottom: 8 }}>Open Source</h2>
        <p style={{ fontSize: 13, color: '#666' }}>
          Built with Tauri 2, React 19, TanStack Router, and the Reflect Agent core.
        </p>
      </section>
    </div>
  );
}
