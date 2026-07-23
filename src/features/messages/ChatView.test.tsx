/**
 * ChatView component tests — verify session loading/empty/error/retry states.
 *
 * Mocks @tanstack/react-router `useParams` to control the active sessionId
 * and `reflect_replay_session` to drive loading / error / success flows.
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
  { type: 'user_input', turn_id: 't1', text: 'hello there' },
  { type: 'agent_message', turn_id: 't1', text: 'hi! how can I help?' },
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

    // Loading banner should appear while the replay is pending.
    await waitFor(() => {
      expect(screen.getByText(/^Loading session…$/)).toBeDefined();
    });

    // Resolve the replay and verify hydration.
    await act(async () => {
      resolveReplay(SAMPLE_RECORDS);
    });
    await waitFor(() => {
      const state = useAgentStore.getState() as any;
      expect(state.loadedSessionId).toBe('sess-42');
      expect(state.turns.length).toBeGreaterThan(0);
    });

    // After hydration, the Viewing banner must render and loading must clear.
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