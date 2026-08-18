/**
 * ChatView 组件测试 —— 验证会话加载 / 空态 / 错误 / 重试状态。
 *
 * 模拟 @tanstack/react-router 的 `useParams` 控制活动 sessionId，
 * 并模拟 `reflect_replay_session` 驱动加载 / 错误 / 成功流程。
 */
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { mockInvoke, resetMockInvoke, createTestQueryClient } from '@/test/setup';
import { useAgentStore } from '@/stores/agentStore';
import { I18nProvider, saveLocale } from '@/utils/i18n';

vi.mock('@tanstack/react-router', () => ({
  useParams: () => ({ sessionId: window.__chatTestSessionId }),
  useNavigate: () => () => {},
  useLocation: () => ({ pathname: window.__chatTestPath ?? '/chat' }),
}));

declare global {
  interface Window {
    __chatTestSessionId?: string;
    __chatTestPath?: string;
  }
}

import { ChatView } from './ChatView';

const SAMPLE_RECORDS = [
  {
    seq: 1,
    kind: 'event' as const,
    timestamp: 0,
    payload: { id: 't1', msg: { type: 'agent_message', text: 'hi! how can I help?' } },
  },
];

function renderWithProviders(ui: React.ReactElement) {
  const qc = createTestQueryClient();
  return render(
    <I18nProvider>
      <QueryClientProvider client={qc}>{ui}</QueryClientProvider>
    </I18nProvider>,
  );
}

describe('ChatView', () => {
  beforeEach(() => {
    resetMockInvoke();
    saveLocale('en');
    window.__chatTestSessionId = undefined;
    window.__chatTestPath = '/chat';
    useAgentStore.setState({
      turns: [],
      loadedSessionId: null,
      pendingApprovals: [],
      pendingQuestions: [],
      pendingAskUser: [],
      pendingPlan: null,
    } as any);
  });
  afterEach(() => {
    cleanup();
  });

  it('shows loading state, then hydrates turns on successful replay', async () => {
    let resolveReplay!: (records: typeof SAMPLE_RECORDS) => void;
    mockInvoke('reflect_replay_session', async () => new Promise((resolve) => { resolveReplay = resolve; }));
    window.__chatTestSessionId = 'sess-42';
    window.__chatTestPath = '/chat/sess-42';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });

    // 回放挂起期间应显示加载横幅。
    await waitFor(() => {
      expect(screen.getByText(/^Loading session…$/)).toBeDefined();
    });

    // 解决回放并验证本地状态水合。
    await act(async () => {
      resolveReplay(SAMPLE_RECORDS);
    });
    await waitFor(() => {
      const state = useAgentStore.getState() as any;
      expect(state.loadedSessionId).toBe('sess-42');
      expect(state.turns.length).toBeGreaterThan(0);
    });

    // 水合后，Viewing 横幅必须渲染，加载状态必须清除。
    await waitFor(() => {
      expect(screen.getByText(/Viewing session/)).toBeDefined();
    });
    expect(screen.queryByText(/^Loading session…$/)).toBeNull();
  });

  it('shows empty session state when replay returns no records', async () => {
    mockInvoke('reflect_replay_session', async () => []);
    window.__chatTestSessionId = 'sess-empty';
    window.__chatTestPath = '/chat/sess-empty';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });
    await waitFor(() => {
      expect(screen.getByText(/This session has no messages/)).toBeDefined();
    });
  });

  it('shows localized error and recovers via Retry button', async () => {
    let attempts = 0;
    mockInvoke('reflect_replay_session', async () => {
      attempts += 1;
      if (attempts === 1) throw new Error('replay failed');
      return SAMPLE_RECORDS;
    });
    window.__chatTestSessionId = 'sess-fail';
    window.__chatTestPath = '/chat/sess-fail';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });
    await waitFor(() => {
      expect(screen.getByText(/Could not load this session/)).toBeDefined();
    });

    const retry = await screen.findByText(/^Retry$/);
    await act(async () => {
      fireEvent.click(retry);
    });
    await waitFor(() => {
      const state = useAgentStore.getState() as any;
      expect(state.loadedSessionId).toBe('sess-fail');
      expect(state.turns.length).toBeGreaterThan(0);
    });
  });

  it('renders Chinese copy after switching locale to zh-CN', async () => {
    saveLocale('zh-CN');
    mockInvoke('reflect_replay_session', async () => []);
    window.__chatTestSessionId = 'sess-zh';
    window.__chatTestPath = '/chat/sess-zh';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });
    await waitFor(() => {
      expect(screen.getByText('此会话没有消息。')).toBeDefined();
    });
  });

  it('clears the loaded session when navigating to /chat (no id)', async () => {
    mockInvoke('reflect_replay_session', async () => SAMPLE_RECORDS);
    window.__chatTestSessionId = 'sess-1';
    window.__chatTestPath = '/chat/sess-1';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });
    await waitFor(() => {
      expect((useAgentStore.getState() as any).loadedSessionId).toBe('sess-1');
    });

    cleanup();
    window.__chatTestSessionId = undefined;
    window.__chatTestPath = '/chat';

    await act(async () => {
      renderWithProviders(<ChatView />);
    });

    await waitFor(() => {
      expect((useAgentStore.getState() as any).loadedSessionId).toBeNull();
    });
  });
});