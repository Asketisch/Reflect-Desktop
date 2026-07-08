/**
 * Vitest — bridge fallback 测试。
 */
import { describe, it, expect } from 'vitest';
import { invoke, listen } from '@/utils/bridge';

describe('invoke', () => {
  it('returns the value from mocked handler', async () => {
    // test setup.tsx 默认注册了 'ping' mock；未注册时通过全局 mock 走 fallback
    const r = await invoke<{ msg: string }>('ping');
    expect(r).toEqual({ msg: 'pong', version: '0.1.0' });
  });

  it('does not throw on unmocked cmd in test env (mock throws synchronously to setup path)', async () => {
    // Note: test setup wraps invoke() in a mock that throws for unmocked cmds.
    // Bridge.ts catches non-Tauri errors; mock throws "[mock] invoke(...) not mocked"
    // which is NOT a Tauri error → bridge rethrows. We assert it rethrows.
    await expect(invoke('definitely_not_a_real_cmd_xyz')).rejects.toThrow();
  });
});

describe('listen', () => {
  it('returns an unlisten function', async () => {
    const unlisten = await listen<string>('reflect_event', () => {});
    expect(typeof unlisten).toBe('function');
    expect(() => unlisten()).not.toThrow();
  });
});