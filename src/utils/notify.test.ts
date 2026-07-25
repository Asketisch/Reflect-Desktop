import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { unsubscribeMock, subscribeAgentEventMock } = vi.hoisted(() => {
  const unsubscribeMock = vi.fn();
  return {
    unsubscribeMock,
    subscribeAgentEventMock: vi.fn(() => unsubscribeMock),
  };
});

vi.mock('@/services/agentEventBus', () => ({
  subscribeAgentEvent: subscribeAgentEventMock,
}));

import {
  loadNotifyOptions,
  saveNotifyOptions,
  useAgentNotifications,
  type AgentNotifyOptions,
} from '@/utils/notify';

const STORAGE_KEY = 'reflect.notify.options';

describe('notify options', () => {
  beforeEach(() => {
    localStorage.clear();
    subscribeAgentEventMock.mockClear();
    unsubscribeMock.mockClear();
  });

  it('normalizes stored partial options with defaults', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ sound: true }));

    expect(loadNotifyOptions()).toEqual({
      sound: true,
      system: false,
      volume: 0.4,
    });
  });

  it('falls back to normalized defaults for invalid storage', () => {
    localStorage.setItem(STORAGE_KEY, 'not-json');

    expect(loadNotifyOptions()).toEqual({
      sound: false,
      system: false,
      volume: 0.4,
    });
  });

  it('saves options in the storage format loadNotifyOptions consumes', () => {
    const options: AgentNotifyOptions = { sound: false, system: false, volume: 0.2 };

    saveNotifyOptions(options);

    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')).toEqual(options);
    expect(loadNotifyOptions()).toEqual(options);
  });
});

describe('useAgentNotifications', () => {
  beforeEach(() => {
    subscribeAgentEventMock.mockClear();
    unsubscribeMock.mockClear();
  });

  it('subscribes through the shared event bus and cleans up on unmount', () => {
    const options: AgentNotifyOptions = { sound: false, system: false, volume: 0.4 };
    const { unmount } = renderHook(() => useAgentNotifications(options));

    expect(subscribeAgentEventMock).toHaveBeenCalledOnce();
    expect(subscribeAgentEventMock).toHaveBeenCalledWith(expect.any(Function));
    expect(unsubscribeMock).not.toHaveBeenCalled();

    unmount();

    expect(unsubscribeMock).toHaveBeenCalledOnce();
  });
});
