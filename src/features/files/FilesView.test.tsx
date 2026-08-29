/**
 * FilesView —— LSP 开关 / 预热链路测试。
 *
 * 覆盖:chip 默认关闭、点击开启调 reflect_lsp_set_enabled(true) 并持久化
 * 偏好、开启状态下打开代码文件触发 reflect_lsp_warmup。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { I18nProvider } from '@/utils/i18n';
import { mockInvoke, renderWithQueryClient } from '@/test/setup';
import { FilesView, lspPrefKey } from './FilesView';

const WORKSPACE = '/tmp/demo-ws';

function renderView() {
  return renderWithQueryClient(
    <I18nProvider>
      <FilesView />
    </I18nProvider>,
  );
}

describe('FilesView LSP', () => {
  beforeEach(() => {
    localStorage.clear();
    mockInvoke('reflect_agent_status', async () => ({
      ready: true,
      has_model: true,
      model: 'openai/x',
      workspace: WORKSPACE,
      degraded_reason: null,
    }));
    mockInvoke('reflect_list_dir', async () => ({
      cwd: WORKSPACE,
      entries: [
        { name: 'a.ts', path: `${WORKSPACE}/a.ts`, kind: 'file', size: 10, mtime: 0, depth: 0 },
      ],
    }));
    mockInvoke('reflect_read_file', async () => ({
      path: `${WORKSPACE}/a.ts`,
      content: 'const x = 1;\n',
      size: 13,
      binary: false,
      truncated: false,
    }));
  });

  it('chip 默认关闭;点击开启 → set_enabled(true) + 偏好持久化', async () => {
    const setCalls: boolean[] = [];
    let enabled = false;
    mockInvoke('reflect_lsp_status', async () => ({ enabled, servers: [] }));
    mockInvoke('reflect_lsp_set_enabled', async (_cmd: string, args?: { enabled: boolean }) => {
      enabled = args?.enabled ?? false;
      setCalls.push(enabled);
      return { enabled, servers: enabled ? ['rust-analyzer'] : [] };
    });
    mockInvoke('reflect_lsp_warmup', async () => ({ warmed: true, server: null }));
    renderView();

    const chip = await screen.findByTestId('files-lsp-toggle');
    await waitFor(() => expect(chip.textContent).toContain('LSP off'));
    fireEvent.click(chip);
    await waitFor(() => {
      expect(setCalls).toEqual([true]);
      expect(localStorage.getItem(lspPrefKey(WORKSPACE))).toBe('on');
    });
    await waitFor(() => expect(chip.textContent).toContain('LSP on'));
  });

  it('开启状态下打开代码文件 → 触发 warmup', async () => {
    const warmupCalls: string[] = [];
    mockInvoke('reflect_lsp_status', async () => ({ enabled: true, servers: ['rust-analyzer'] }));
    mockInvoke('reflect_lsp_set_enabled', async () => ({ enabled: true, servers: [] }));
    mockInvoke('reflect_lsp_warmup', async (_cmd: string, args?: { path: string }) => {
      warmupCalls.push(args?.path ?? '');
      return { warmed: true, server: 'rust-analyzer' };
    });
    localStorage.setItem(lspPrefKey(WORKSPACE), 'on');
    renderView();

    const fileBtn = await screen.findByText('a.ts');
    fireEvent.click(fileBtn);
    await waitFor(() => expect(screen.getByTestId('cm-editor')).toBeTruthy());
    await waitFor(() => expect(warmupCalls).toEqual([`${WORKSPACE}/a.ts`]));
  });

  it('关闭状态下打开文件不触发 warmup', async () => {
    const warmup = vi.fn(async () => ({ warmed: false, server: null }));
    mockInvoke('reflect_lsp_status', async () => ({ enabled: false, servers: [] }));
    mockInvoke('reflect_lsp_set_enabled', async () => ({ enabled: false, servers: [] }));
    mockInvoke('reflect_lsp_warmup', warmup);
    renderView();

    const fileBtn = await screen.findByText('a.ts');
    fireEvent.click(fileBtn);
    await waitFor(() => expect(screen.getByTestId('cm-editor')).toBeTruthy());
    // 给 effect 一拍执行窗口。
    await waitFor(() => expect(screen.getByTestId('files-lsp-toggle')).toBeTruthy());
    expect(warmup).not.toHaveBeenCalled();
  });
});
