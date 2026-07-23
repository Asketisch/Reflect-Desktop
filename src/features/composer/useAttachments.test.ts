/**
 * Vitest — useAttachments hook.
 */
import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAttachments } from './useAttachments';

describe('useAttachments', () => {
  it('starts empty', () => {
    const { result } = renderHook(() => useAttachments());
    expect(result.current.attachments).toEqual([]);
  });

  it('addLocalImage appends', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addLocalImage('/tmp/a.png'));
    expect(result.current.attachments).toHaveLength(1);
    expect(result.current.attachments[0]).toEqual({ kind: 'local_image', path: '/tmp/a.png' });
  });

  it('addInlineImage appends with mime', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addInlineImage('data:image/png;base64,abc', 'image/png'));
    expect(result.current.attachments[0]).toEqual({
      kind: 'image',
      data: 'data:image/png;base64,abc',
      mime_type: 'image/png',
    });
  });

  it('addSkillMention appends skill', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => result.current.addSkillMention('code-review'));
    expect(result.current.attachments[0]).toEqual({ kind: 'skill', name: 'code-review' });
  });

  it('remove(idx) drops entry', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => {
      result.current.addLocalImage('/a');
      result.current.addLocalImage('/b');
    });
    act(() => result.current.remove(0));
    expect(result.current.attachments).toHaveLength(1);
    expect(result.current.attachments[0]).toMatchObject({ path: '/b' });
  });

  it('clear empties all', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => {
      result.current.addLocalImage('/a');
      result.current.addSkillMention('foo');
    });
    act(() => result.current.clear());
    expect(result.current.attachments).toHaveLength(0);
  });

  it('toUserInputItems converts each kind', () => {
    const { result } = renderHook(() => useAttachments());
    act(() => {
      result.current.addLocalImage('/x.png');
      result.current.addInlineImage('d', 'image/png');
      result.current.addSkillMention('s');
    });
    const items = result.current.toUserInputItems();
    expect(items).toEqual([
      { type: 'local_image', path: '/x.png' },
      { type: 'image', data: 'd', mime_type: 'image/png' },
      { type: 'skill', name: 's' },
    ]);
  });
});
