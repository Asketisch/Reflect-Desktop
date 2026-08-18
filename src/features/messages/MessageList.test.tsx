/**
 * Vitest — MessageList 组件测试(阶段 3b:items 模型)。
 *
 * MessageList 直接从 useAgentStore 读 state,测试通过 store API 注入 state。
 *
 * 验证:
 * 1. 渲染空状态
 * 2. 显示 session 等待信息
 * 3. 渲染 user_text + assistant_text items
 * 4. 渲染 tool_call + error items
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { MessageList } from '@/features/messages/MessageList';
import { createTestQueryClient } from '@/test/setup.tsx';
import { useAgentStore } from '@/stores/agentStore';

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('MessageList', () => {
  beforeEach(() => {
    useAgentStore.getState().reset();
  });
  afterEach(() => {
    cleanup();
  });

  it('renders empty state initially', () => {
    render(wrap(<MessageList />));
    expect(screen.getByText(/Start a conversation/i)).toBeDefined();
  });

  it('shows empty-state hint when no turns (no session yet)', () => {
    render(wrap(<MessageList />));
    expect(screen.getByText(/Start a conversation/i)).toBeDefined();
  });

  it('renders user_text + assistant_text items', () => {
    useAgentStore.setState({
      turns: [
        {
          id: 't1',
          status: 'done',
          items: [
            { kind: 'user_text', text: 'hello' },
            { kind: 'assistant_text', text: 'Hi there!', streaming: false },
          ],
        },
      ],
      session: { model: 'stub/test', provider: 'local' },
    });
    render(wrap(<MessageList />));
    expect(screen.getByText('hello')).toBeDefined();
    expect(screen.getByText('Hi there!')).toBeDefined();
  });

  it('renders tool_call row with status badge', () => {
    useAgentStore.setState({
      turns: [
        {
          id: 't2',
          status: 'streaming',
          items: [
            { kind: 'user_text', text: 'list files' },
            {
              kind: 'tool_call',
              toolName: 'bash',
              callId: 'c1',
              argsSummary: 'ls',
              status: 'done',
            },
          ],
        },
      ],
    });
    render(wrap(<MessageList />));
    // tool_call 行使用 <ToolCell>，它在独立 span 中渲染工具名与摘要。
    const cell = document.querySelector('[data-tool="bash"]');
    expect(cell).not.toBeNull();
    expect(cell!.getAttribute('data-status')).toBe('done');
  });

  it('renders error item as red banner', () => {
    useAgentStore.setState({
      turns: [
        {
          id: 't3',
          status: 'done',
          items: [
            { kind: 'user_text', text: 'go' },
            { kind: 'error', text: 'something broke' },
          ],
        },
      ],
    });
    render(wrap(<MessageList />));
    expect(screen.getByText(/something broke/)).toBeDefined();
  });
});
