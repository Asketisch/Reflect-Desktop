/**
 * Vitest — CodeEditor (B9-01).
 *
 * 不引入完整 prism DOM 比对;只验证头部 + 行号 + 二进制提示。
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CodeEditor } from './CodeEditor';

describe('CodeEditor', () => {
  it('renders the path + detected language', () => {
    render(<CodeEditor path="/x/main.ts" content="const x = 1;\nconst y = 2;" />);
    const editor = screen.getByTestId('code-editor');
    expect(editor.getAttribute('data-lang')).toBe('typescript');
    expect(screen.getByText('/x/main.ts')).toBeDefined();
  });

  it('shows line numbers (one per line)', () => {
    render(<CodeEditor path="/x/a.txt" content={'a\nb\nc\nd'} />);
    // 4 line numbers expected (1..4)
    expect(screen.getByText('1')).toBeDefined();
    expect(screen.getByText('4')).toBeDefined();
  });

  it('shows truncated badge when file is clipped', () => {
    render(<CodeEditor path="/x/big.txt" content="x" truncated />);
    expect(screen.getByText(/clipped to 1 MiB/i)).toBeDefined();
  });

  it('renders binary notice for binary files', () => {
    render(<CodeEditor path="/x/blob.bin" content="" binary />);
    expect(screen.getByTestId('code-editor-binary')).toBeDefined();
  });

  it('falls back to plaintext for unknown extensions', () => {
    render(<CodeEditor path="/x/foo.unknownext" content="hi" />);
    expect(screen.getByTestId('code-editor').getAttribute('data-lang')).toBe('plaintext');
  });
});