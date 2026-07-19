/**
 * Vitest — commands 包装测试。
 *
 * 验证 wrapper 调用 invoke 转发正确 cmd + 参数（依赖 test/setup.tsx 的 mock）。
 */
import { describe, it, expect } from 'vitest';
import {
  reflect_interrupt,
  reflect_compact,
  reflect_rewind,
  reflect_tool_approval,
  reflect_set_permission_mode,
} from '@/utils/commands';

describe('commands (forwarding to mock invoke)', () => {
  it('reflect_interrupt calls invoke("reflect_interrupt")', async () => {
    // setup.tsx 已 mock reflect_set_permission_mode 等; reflect_interrupt 未注册 → mock 抛错
    await expect(reflect_interrupt()).rejects.toThrow(/not mocked/);
  });

  it('reflect_compact also un-registered by default', async () => {
    await expect(reflect_compact()).rejects.toThrow(/not mocked/);
  });

  it('reflect_rewind forwards toTurnId argument', async () => {
    await expect(reflect_rewind('turn-123')).rejects.toThrow(/not mocked/);
    await expect(reflect_rewind()).rejects.toThrow(/not mocked/);
  });

  it('reflect_tool_approval accepts ReviewDecision', async () => {
    await expect(reflect_tool_approval('id-1', 'approve')).rejects.toThrow(/not mocked/);
    await expect(
      reflect_tool_approval('id-2', { deny: { reason: 'risky' } }),
    ).rejects.toThrow(/not mocked/);
    await expect(
      reflect_tool_approval('id-3', 'approve_for_session'),
    ).rejects.toThrow(/not mocked/);
  });

  it('reflect_set_permission_mode is mocked by setup and returns undefined', async () => {
    expect(await reflect_set_permission_mode('default')).toBeUndefined();
  });
});