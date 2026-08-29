/**
 * summarizeToolOutput 单元测试 —— 结构化 ToolOutput（live 事件与
 * 历史回放同构）的 text / diff / path 提取与兜底行为。
 */
import { describe, expect, it } from 'vitest';
import { summarizeToolOutput } from './turns';

describe('summarizeToolOutput', () => {
  it('passes plain strings through', () => {
    expect(summarizeToolOutput('hello')).toEqual({ text: 'hello' });
  });

  it('returns empty text for null/undefined', () => {
    expect(summarizeToolOutput(null)).toEqual({ text: '' });
    expect(summarizeToolOutput(undefined)).toEqual({ text: '' });
  });

  it('extracts text, diff and path from a structured ToolOutput', () => {
    const output = {
      content: [
        { type: 'text', text: 'wrote 42 bytes' },
        { type: 'diff', unified_diff: '--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-a\n+b' },
      ],
      is_error: false,
      metadata: { path: 'src/f.rs' },
      elapsed_ms: 3,
    };
    expect(summarizeToolOutput(output)).toEqual({
      text: 'wrote 42 bytes',
      diff: '--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-a\n+b',
      path: 'src/f.rs',
    });
  });

  it('joins multiple text blocks and tolerates missing diff/metadata', () => {
    const output = {
      content: [{ type: 'text', text: 'line1' }, { type: 'text', text: 'line2' }],
      is_error: false,
    };
    expect(summarizeToolOutput(output)).toEqual({ text: 'line1\nline2' });
  });

  it('falls back to pretty JSON for unknown object shapes', () => {
    expect(summarizeToolOutput({ foo: 1 })).toEqual({ text: '{\n  "foo": 1\n}' });
  });
});
