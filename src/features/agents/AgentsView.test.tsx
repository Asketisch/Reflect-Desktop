/**
 * Vitest —— AgentsView（Phase 1 条目 3）。
 *
 * 基于模拟 IPC 的冒烟 + 行为测试。验证：
 *   - 页面标题 + 预置行渲染
 *   - New 按钮打开编辑器
 *   - 编辑器 Save 将草稿转发到 reflect_save_agent_def
 *   - 编辑从点击的行预填草稿
 *   - delete 转发到 reflect_delete_agent_def
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { AgentsView } from './AgentsView';
import { createTestQueryClient, resetMockInvoke } from '@/test/setup';

const calls: Array<{ cmd: string; args: unknown }> = [];

vi.mock('@/utils/commands', async () => {
  const actual = await vi.importActual<typeof import('@/utils/commands')>('@/utils/commands');
  return {
    ...actual,
    reflect_list_agent_defs: vi.fn(async () => {
      calls.push({ cmd: 'reflect_list_agent_defs', args: undefined });
      return [
        {
          name: 'reviewer',
          description: 'Reviews code',
          spawnable: true,
          readonly: true,
          tools: ['read', 'grep'],
          disallowed_tools: ['bash'],
          model: 'inherit',
          max_turns: 30,
          memory: ['project'],
          mcp_collections: [],
          system_prompt: 'You are strict.',
        },
      ];
    }),
    reflect_save_agent_def: vi.fn(async (def: Record<string, unknown>) => {
      calls.push({ cmd: 'reflect_save_agent_def', args: { def } });
      return def;
    }),
    reflect_delete_agent_def: vi.fn(async (name: string) => {
      calls.push({ cmd: 'reflect_delete_agent_def', args: { name } });
      return true;
    }),
    reflect_get_agent_def: vi.fn(async () => null),
    reflect_parse_agent_md: vi.fn(async () => null),
  };
});

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('AgentsView', () => {
  beforeEach(() => {
    cleanup();
    calls.length = 0;
    resetMockInvoke();
  });

  it('renders the page title and seeded agent', async () => {
    render(wrap(<AgentsView />));
    expect(screen.getByText('Agents')).toBeDefined();
    await waitFor(() => {
      expect(screen.getByText('Reviews code')).toBeDefined();
    });
  });

  it('opens the editor on New click', async () => {
    render(wrap(<AgentsView />));
    fireEvent.click(screen.getByTestId('agents-add-btn'));
    expect(screen.getByTestId('agent-editor')).toBeDefined();
    expect(screen.getByTestId('agent-editor-name')).toBeDefined();
  });

  it('submits save with name + description required', async () => {
    render(wrap(<AgentsView />));
    fireEvent.click(screen.getByTestId('agents-add-btn'));
    fireEvent.change(screen.getByTestId('agent-editor-name'), {
      target: { value: 'tester' },
    });
    fireEvent.change(screen.getByTestId('agent-editor-description'), {
      target: { value: 'A test agent' },
    });
    fireEvent.click(screen.getByTestId('agent-editor-save'));
    await waitFor(() => {
      expect(
        calls.some(
          (c) =>
            c.cmd === 'reflect_save_agent_def' &&
            (c.args as { def: { name: string } }).def.name === 'tester',
        ),
      ).toBe(true);
    });
  });

  it('pre-fills the editor when editing a seeded row', async () => {
    render(wrap(<AgentsView />));
    await waitFor(() => screen.getByTestId('agent-edit-reviewer'));
    fireEvent.click(screen.getByTestId('agent-edit-reviewer'));
    const nameInput = screen.getByTestId('agent-editor-name') as HTMLInputElement;
    const descInput = screen.getByTestId('agent-editor-description') as HTMLInputElement;
    expect(nameInput.value).toBe('reviewer');
    expect(descInput.value).toBe('Reviews code');
    // 编辑时 name 被禁用（重命名会改变文件名）。
    expect(nameInput.disabled).toBe(true);
  });

  it('forwards delete to reflect_delete_agent_def', async () => {
    render(wrap(<AgentsView />));
    await waitFor(() => screen.getByTestId('agent-delete-reviewer'));
    fireEvent.click(screen.getByTestId('agent-delete-reviewer'));
    await waitFor(() => {
      expect(
        calls.some(
          (c) =>
            c.cmd === 'reflect_delete_agent_def' &&
            (c.args as { name: string }).name === 'reviewer',
        ),
      ).toBe(true);
    });
  });
});
