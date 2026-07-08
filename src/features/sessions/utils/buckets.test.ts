/**
 * Vitest — buckets 纯函数测试。
 */
import { describe, it, expect } from 'vitest';
import {
  bucketFor,
  bucketSessions,
  displayTitle,
  SESSION_BUCKET_LABELS,
} from '@/features/sessions/utils/buckets';
import type { ReflectSessionInfo } from '@/utils/tauri';

function makeSession(overrides: Partial<ReflectSessionInfo> = {}): ReflectSessionInfo {
  return {
    session_id: 'sess-' + Math.random().toString(36).slice(2, 8),
    thread_id: 'thread-1',
    model: 'stub/test',
    provider: 'local',
    started_at: new Date().toISOString(),
    message_count: 0,
    tool_count: 0,
    token_total: 0,
    cwd: '/tmp',
    display_name: undefined,
    ...overrides,
  };
}

const NOW = new Date('2026-07-08T12:00:00Z').getTime();
const HOUR = 3600 * 1000;
const DAY = 24 * HOUR;

describe('bucketFor', () => {
  it('classifies <1h as Now', () => {
    const s = makeSession({ started_at: new Date(NOW - 30 * 60 * 1000).toISOString() });
    expect(bucketFor(s, NOW)).toBe('Now');
  });

  it('classifies 1h-24h as Today', () => {
    const s = makeSession({ started_at: new Date(NOW - 5 * HOUR).toISOString() });
    expect(bucketFor(s, NOW)).toBe('Today');
  });

  it('classifies 24-48h as Yesterday', () => {
    const s = makeSession({ started_at: new Date(NOW - 1.5 * DAY).toISOString() });
    expect(bucketFor(s, NOW)).toBe('Yesterday');
  });

  it('classifies 2-7d as This week', () => {
    const s = makeSession({ started_at: new Date(NOW - 4 * DAY).toISOString() });
    expect(bucketFor(s, NOW)).toBe('This week');
  });

  it('classifies >7d as Older', () => {
    const s = makeSession({ started_at: new Date(NOW - 30 * DAY).toISOString() });
    expect(bucketFor(s, NOW)).toBe('Older');
  });

  it('falls back to Older on invalid date', () => {
    const s = makeSession({ started_at: 'not-a-date' });
    expect(bucketFor(s, NOW)).toBe('Older');
  });
});

describe('bucketSessions', () => {
  it('returns empty for empty input', () => {
    expect(bucketSessions([], NOW)).toEqual([]);
  });

  it('groups sessions and preserves order', () => {
    const now = makeSession({ session_id: 'a', started_at: new Date(NOW - 30 * 60 * 1000).toISOString() });
    const today = makeSession({ session_id: 'b', started_at: new Date(NOW - 5 * HOUR).toISOString() });
    const older = makeSession({ session_id: 'c', started_at: new Date(NOW - 30 * DAY).toISOString() });
    const buckets = bucketSessions([older, now, today], NOW);
    const labels = buckets.map((b) => b.label);
    // Order: Now → Today → ... → Older
    expect(labels).toEqual(['Now', 'Today', 'Older']);
    expect(buckets[0].sessions[0].session_id).toBe('a');
    expect(buckets[1].sessions[0].session_id).toBe('b');
    expect(buckets[2].sessions[0].session_id).toBe('c');
  });

  it('omits empty buckets', () => {
    const now = makeSession({ session_id: 'a', started_at: new Date(NOW - 30 * 60 * 1000).toISOString() });
    const buckets = bucketSessions([now], NOW);
    expect(buckets.map((b) => b.label)).toEqual(['Now']);
  });

  it('exposes canonical label order', () => {
    expect(SESSION_BUCKET_LABELS).toEqual(['Now', 'Today', 'Yesterday', 'This week', 'Older']);
  });
});

describe('displayTitle', () => {
  it('returns display_name when set', () => {
    expect(displayTitle({ session_id: 'abc', display_name: 'My chat' })).toBe('My chat');
  });

  it('falls back to id prefix', () => {
    expect(displayTitle({ session_id: 'abcdef12-3456', display_name: undefined })).toBe('abcdef12');
  });
});