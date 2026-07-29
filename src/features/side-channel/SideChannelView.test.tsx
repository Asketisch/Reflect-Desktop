/**
 * Vitest — SideChannelView (Phase 2 item 1).
 *
 * Smoke + behavior tests against mocked IPC:
 *   - page title + status badge render
 *   - Start form opens on click; submits with agent_name + prompt
 *   - running channels expose a Cancel button that forwards to
 *     reflect_cancel_side_channel
 *   - terminal channels (done/cancelled/error) hide the Cancel button
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { SideChannelView } from './SideChannelView';
import { createTestQueryClient, resetMockInvoke } from '@/test/setup';

const calls: Array<{ cmd: string; args: unknown }> = [];

vi.mock('@/utils/commands', async () => {
  const actual = await vi.importActual<typeof import('@/utils/commands')>('@/utils/commands');
  return {
    ...actual,
    reflect_list_side_channels: vi.fn(async () => {
      calls.push({ cmd: 'reflect_list_side_channels', args: undefined });
      return [
        {
          id: 'side-running01',
          agent_name: 'reviewer',
          prompt: 'audit PR',
          started_at_ms: 1000,
          status: 'running',
          duration_ms: null,
        },
        {
          id: 'side-done0002',
          agent_name: 'default',
          prompt: 'echo hi',
          started_at_ms: 500,
          status: 'done',
          duration_ms: 1234,
        },
      ];
    }),
    reflect_start_side_channel: vi.fn(async (args: Record<string, unknown>) => {
      calls.push({ cmd: 'reflect_start_side_channel', args });
      return {
        id: 'side-new0003',
        agent_name: args.agentName as string,
        prompt: args.prompt as string,
        started_at_ms: 2000,
      };
    }),
    reflect_cancel_side_channel: vi.fn(async (id: string) => {
      calls.push({ cmd: 'reflect_cancel_side_channel', args: { id } });
      return true;
    }),
    reflect_get_side_channel: vi.fn(async () => null),
  };
});

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('SideChannelView', () => {
  beforeEach(() => {
    cleanup();
    calls.length = 0;
    resetMockInvoke();
  });

  it('renders the page title and running count badge', async () => {
    render(wrap(<SideChannelView />));
    expect(screen.getByText('Side-channels')).toBeDefined();
    await waitFor(() => {
      expect(screen.getByText('1 running')).toBeDefined();
    });
  });

  it('renders running + done rows', async () => {
    render(wrap(<SideChannelView />));
    await waitFor(() => {
      expect(screen.getByText('side-running01')).toBeDefined();
      expect(screen.getByText('audit PR')).toBeDefined();
      expect(screen.getByText('echo hi')).toBeDefined();
    });
  });

  it('running row exposes Cancel; terminal rows do not', async () => {
    render(wrap(<SideChannelView />));
    await waitFor(() => screen.getByTestId('side-channel-row-side-running01'));
    // Running row has a Cancel button.
    expect(screen.getByTestId('side-channel-cancel-side-running01')).toBeDefined();
    // Done row does not (no Cancel button for terminal status).
    expect(screen.queryByTestId('side-channel-cancel-side-done0002')).toBeNull();
  });

  it('opens the start form and submits with agent_name + prompt', async () => {
    render(wrap(<SideChannelView />));
    // Wait for the list to render so the Add button isn't competing with
    // any post-mount updates from refetchInterval.
    await waitFor(() => screen.getByTestId('side-channel-row-side-running01'));
    fireEvent.click(screen.getByTestId('side-channel-add-btn'));
    expect(screen.getByTestId('side-channel-start-form')).toBeDefined();

    fireEvent.change(screen.getByTestId('side-channel-start-name'), {
      target: { value: 'reviewer' },
    });
    fireEvent.change(screen.getByTestId('side-channel-start-prompt'), {
      target: { value: 'double-check the diff' },
    });
    // Click the submit button so onClick (which calls start on each render)
    // fires after the latest state.
    fireEvent.click(screen.getByTestId('side-channel-start-submit'));

    await waitFor(
      () => {
        const startCall = calls.find((c) => c.cmd === 'reflect_start_side_channel');
        expect(startCall).toBeDefined();
        expect((startCall?.args as { agent_name: string }).agent_name).toBe('reviewer');
        expect((startCall?.args as { prompt: string }).prompt).toBe('double-check the diff');
      },
      { timeout: 2000 },
    );
  });

  it('Cancel click forwards reflect_cancel_side_channel', async () => {
    render(wrap(<SideChannelView />));
    await waitFor(() => screen.getByTestId('side-channel-cancel-side-running01'));
    fireEvent.click(screen.getByTestId('side-channel-cancel-side-running01'));
    await waitFor(() => {
      expect(
        calls.some(
          (c) =>
            c.cmd === 'reflect_cancel_side_channel' &&
            (c.args as { id: string }).id === 'side-running01',
        ),
      ).toBe(true);
    });
  });
});
