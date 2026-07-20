/**
 * Vitest — SettingsView 组件测试(阶段 4:真实 config 持久化)。
 *
 * 验证:
 * 1. 渲染 Provider / Permissions / Advanced sections
 * 2. 显示 agent 状态徽标(从 reflect_agent_status)
 * 3. permission 按钮点击调 reflect_set_permission_mode
 * 4. Save 按钮调 reflect_save_config
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { SettingsView } from '@/features/settings/SettingsView';
import {
  mockInvoke,
  resetMockInvoke,
  createTestQueryClient,
} from '@/test/setup.tsx';

function renderSettingsView() {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>
      <SettingsView />
    </QueryClientProvider>,
  );
}

const SAMPLE_TOML = `[active]
provider = "anthropic"

[anthropic]
api_key = "sk-ant-existing"
model = "claude-3-5-sonnet-latest"
`;

describe('SettingsView', () => {
  beforeEach(() => {
    resetMockInvoke();
    mockInvoke('reflect_set_permission_mode', async () => {});
    mockInvoke('reflect_save_config', async () => {});
    mockInvoke('reflect_get_config', async () => SAMPLE_TOML);
    mockInvoke('reflect_agent_status', async () => ({
      ready: true,
      has_model: true,
      model: 'anthropic/claude-3-5-sonnet-latest',
      workspace: '/tmp',
      degraded_reason: null,
    }));
  });
  afterEach(() => cleanup());

  it('renders Provider / Permissions / Advanced sections', async () => {
    renderSettingsView();
    expect(screen.getByText('Settings')).toBeDefined();
    // nav + section 都有同名标题，用 getAllByText。
    expect(screen.getAllByText('Provider').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Permissions').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText(/Advanced/).length).toBeGreaterThanOrEqual(1);
  });

  it('shows agent ready status badge when has_model', async () => {
    renderSettingsView();
    await waitFor(() => {
      expect(screen.getByText(/Agent ready/)).toBeDefined();
    });
  });

  it('clicking permission button calls reflect_set_permission_mode', async () => {
    let modeArg: string | null = null;
    mockInvoke('reflect_set_permission_mode', async (_cmd: string, args: unknown) => {
      modeArg = (args as { mode: string }).mode;
    });

    renderSettingsView();
    // permission 按钮文案:auto/prompt/deny/plan。
    const planBtn = await screen.findByText('plan');
    fireEvent.click(planBtn);

    await waitFor(() => expect(modeArg).toBe('plan'), { timeout: 2000 });
  });

  it('Save button calls reflect_save_config with merged TOML', async () => {
    let savedToml: string | null = null;
    mockInvoke('reflect_save_config', async (_cmd: string, args: unknown) => {
      savedToml = (args as { toml: string }).toml;
    });

    renderSettingsView();
    // 等 config 加载完成(状态徽标出现 = get_config + agent_status 都 resolve)。
    await waitFor(() => {
      expect(screen.getByText(/Agent ready/)).toBeDefined();
    });
    // 再等一帧让 useEffect 把 rawToml 写入。
    await waitFor(() => {
      expect(screen.getByText('Save to ~/.reflect/config.toml')).toBeDefined();
    });

    fireEvent.click(screen.getByText('Save to ~/.reflect/config.toml'));
    await waitFor(() => expect(savedToml).not.toBeNull(), { timeout: 2000 });
    // 合并后的 TOML 应保留 active + anthropic 段(SAMPLE_TOML 的内容)。
    expect(savedToml).toContain('[active]');
    expect(savedToml).toContain('provider = "anthropic"');
    expect(savedToml).toContain('[anthropic]');
    expect(savedToml).toContain('sk-ant-existing');
  });
});
