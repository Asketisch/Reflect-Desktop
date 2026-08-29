/**
 * editorStore 单元测试 —— pending 变更累计 / Accept / Reject(inverse-patch
 * + reflect_write_file 落盘)。
 */
import { describe, expect, it, beforeEach } from 'vitest';
import { useEditorStore } from './editorStore';
import { mockInvoke } from '@/test/setup';

const DIFF = `@@ -1,3 +1,3 @@
-a
+AA
 b
`;

function resetStore() {
  useEditorStore.setState({ pending: {}, lastEditedPath: null, contentVersion: {} });
}

describe('editorStore', () => {
  beforeEach(() => {
    mockInvoke('reflect_write_file', async () => '/ws/file.ts');
    resetStore();
  });

  it('recordEdit 解析 unified diff 并累计 hunk', () => {
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    const pending = useEditorStore.getState().pending;
    expect(pending['file.ts']?.hunks).toHaveLength(1);
    expect(pending['file.ts']?.hunks[0].added).toEqual(['AA']);
    expect(useEditorStore.getState().lastEditedPath).toBe('file.ts');
  });

  it('无 hunk 的 diff 不产生 pending', () => {
    useEditorStore.getState().recordEdit('file.ts', 'no hunks here');
    expect(useEditorStore.getState().pending['file.ts']).toBeUndefined();
  });

  it('acceptHunk 清除标记但不动内容;acceptAll 清空', () => {
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    useEditorStore.getState().acceptHunk('file.ts', 0);
    expect(useEditorStore.getState().pending['file.ts']).toBeUndefined();
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    useEditorStore.getState().acceptAll('file.ts');
    expect(useEditorStore.getState().pending['file.ts']).toBeUndefined();
  });

  it('rejectHunk:inverse-patch → reflect_write_file → 清除该块', async () => {
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    const next = await useEditorStore.getState().rejectHunk('file.ts', 0, 'AA\nb\n');
    expect(next).toBe('a\nb\n');
    expect(useEditorStore.getState().pending['file.ts']).toBeUndefined();
  });

  it('rejectHunk 锚定失败返回 null 且不写盘', async () => {
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    const next = await useEditorStore.getState().rejectHunk('file.ts', 0, '完全不同的内容\n');
    expect(next).toBeNull();
    // pending 保留,供用户重试 / 手动处理。
    expect(useEditorStore.getState().pending['file.ts']).toBeDefined();
  });

  it('rejectAll:全部 hunk 还原;任一失败整体不落盘', async () => {
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    const next = await useEditorStore.getState().rejectAll('file.ts', 'AA\nb\n');
    expect(next).toBe('a\nb\n');
    useEditorStore.getState().recordEdit('file.ts', DIFF);
    const failed = await useEditorStore.getState().rejectAll('file.ts', '不匹配\n');
    expect(failed).toBeNull();
  });
});
