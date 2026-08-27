import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { I18nProvider } from '@/utils/i18n';
import { createTestQueryClient, mockInvoke } from '@/test/setup';
import { Composer } from './Composer';

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => vi.fn(),
  useLocation: () => ({ pathname: '/chat' }),
}));

function renderComposer() {
  return render(
    <I18nProvider>
      <QueryClientProvider client={createTestQueryClient()}>
        <Composer />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe('Composer prompt-history integration', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.localStorage.setItem(
      'reflect.history.default',
      JSON.stringify(['first prompt', 'latest prompt']),
    );
  });

  it('restores the pre-browse draft when ArrowDown leaves history', () => {
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;

    fireEvent.change(input, { target: { value: 'unfinished draft' } });
    input.setSelectionRange(0, 0);
    fireEvent.keyDown(input, { key: 'ArrowUp' });
    expect(input.value).toBe('latest prompt');

    input.setSelectionRange(input.value.length, input.value.length);
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(input.value).toBe('unfinished draft');
  });

  it('keeps the original draft while browsing multiple history entries', () => {
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;

    fireEvent.change(input, { target: { value: 'original draft' } });
    input.setSelectionRange(0, 0);
    fireEvent.keyDown(input, { key: 'ArrowUp' });
    input.setSelectionRange(0, 0);
    fireEvent.keyDown(input, { key: 'ArrowUp' });
    expect(input.value).toBe('first prompt');

    input.setSelectionRange(input.value.length, input.value.length);
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(input.value).toBe('latest prompt');
    input.setSelectionRange(input.value.length, input.value.length);
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(input.value).toBe('original draft');
  });

  it('does not submit while an IME composition is active', () => {
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: 'composing' } });
    fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
    expect(input.value).toBe('composing');
  });
});

describe('Composer send shortcut', () => {
  // 注意:不要 resetMockInvoke() —— setup.ts 的 beforeEach 已注册
  // reflect_list_sessions / reflect_rename_session 等默认 handler
  // (useSessions 的 useQuery 依赖);mockInvoke 本身即覆盖语义。
  //
  // mock invoke 的 handler 签名为 (cmd, args),其中
  // args = { submission } —— 见 src/utils/commands/agent.ts::reflect_submit。
  interface Submission {
    id: string;
    op: { type: string; items: unknown[] };
  }
  function trackSubmit(submitted: Submission[]) {
    mockInvoke('reflect_submit', async (_cmd, args) => {
      const submission = (args as { submission: Submission }).submission;
      submitted.push(submission);
      return submission.id;
    });
  }

  it('submits on plain Enter', async () => {
    const submitted: Submission[] = [];
    trackSubmit(submitted);
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: '你好，发送我' } });

    const prevented = fireEvent.keyDown(input, { key: 'Enter' });

    // Enter 被拦截(不走默认换行)并触发一次提交。
    expect(prevented).toBe(false);
    await waitFor(() => expect(submitted).toHaveLength(1));
    expect(submitted[0].op.type).toBe('user_input');
    expect(submitted[0].op.items).toEqual([{ type: 'text', text: '你好，发送我' }]);
    // 提交后草稿清空。
    expect(input.value).toBe('');
  });

  it('keeps Ctrl/Cmd+Enter working as a send shortcut', async () => {
    const submitted: Submission[] = [];
    trackSubmit(submitted);
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: 'cmd enter' } });

    fireEvent.keyDown(input, { key: 'Enter', metaKey: true });

    await waitFor(() => expect(submitted).toHaveLength(1));
    expect(input.value).toBe('');
  });

  it('does not submit on Shift+Enter (newline path)', () => {
    const submitted: Submission[] = [];
    trackSubmit(submitted);
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: 'line one' } });

    const prevented = fireEvent.keyDown(input, { key: 'Enter', shiftKey: true });

    // Shift+Enter 不拦截 —— 浏览器默认行为插入换行;也不触发提交。
    expect(prevented).toBe(true);
    expect(submitted).toHaveLength(0);
    expect(input.value).toBe('line one');
  });

  it('does not submit on empty Enter', () => {
    const submitted: Submission[] = [];
    trackSubmit(submitted);
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(submitted).toHaveLength(0);
  });
});

describe('Composer @ file mention (v1.x)', () => {
  function mockWorkspaceFiles() {
    mockInvoke('reflect_list_dir', async () => ({
      root: '/tmp/proj',
      entries: [
        { name: 'README.md', path: 'README.md', kind: 'file', size: 10, mtime: 0, depth: 1 },
        { name: 'main.ts', path: 'src/main.ts', kind: 'file', size: 10, mtime: 0, depth: 2 },
        { name: 'src', path: 'src', kind: 'dir', size: 0, mtime: 0, depth: 1 },
      ],
      truncated: false,
    }));
  }

  it('typing @ opens the file picker; picking clears @query and adds a chip', async () => {
    mockWorkspaceFiles();
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    // 空 query → 列出全部文件(按 path 字典序), 便于断言目录被过滤。
    fireEvent.change(input, { target: { value: 'read @' } });

    const picker = await screen.findByTestId('mention-picker');
    expect(picker).toBeDefined();
    expect(screen.getByTestId('mention-file-README.md')).toBeDefined();
    expect(screen.getByTestId('mention-file-main.ts')).toBeDefined();
    // 目录不出现在选项中。
    expect(screen.queryByTestId('mention-file-src')).toBeNull();

    fireEvent.click(screen.getByTestId('mention-file-README.md'));

    // 1. 末尾的 `@` 从草稿清除, 前导空白保留。
    expect(input.value).toBe('read ');
    // 2. 附件条出现 file chip。
    expect(screen.getByTestId('attachment-bar')).toBeDefined();
    expect(screen.getByText(/README\.md/)).toBeDefined();
    // 3. 弹层关闭。
    expect(screen.queryByTestId('mention-picker')).toBeNull();
  });

  it('typing @ with no files shows the empty state', async () => {
    mockInvoke('reflect_list_dir', async () => ({
      root: '/tmp/proj',
      entries: [],
      truncated: false,
    }));
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: '@' } });

    await screen.findByText('No matching files.');
  });

  it('toolbar @ button appends @ and opens the picker directly', async () => {
    mockWorkspaceFiles();
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;

    fireEvent.click(screen.getByTestId('composer-attach-mention'));

    // 按钮直接打开弹层(setState 不触发 onChange, 不能依赖 input 流)。
    await screen.findByTestId('mention-picker');
    expect(input.value).toBe('@');

    // 已有文本且末尾非空白 → 补前导空格。
    fireEvent.change(input, { target: { value: 'hello' } });
    fireEvent.click(screen.getByTestId('composer-attach-mention'));
    expect(input.value).toBe('hello @');
  });

  it('file attachment is sent as a file UserInputItem', async () => {
    const submitted: Array<{ op: { type: string; items: unknown[] } }> = [];
    mockInvoke('reflect_submit', async (_cmd, args) => {
      const s = (args as { submission: { op: { type: string; items: unknown[] } } }).submission;
      submitted.push(s);
      return 'sub-ok';
    });
    mockWorkspaceFiles();
    renderComposer();
    const input = screen.getByRole('textbox', { name: /message reflect/i }) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: 'look @ma' } });
    fireEvent.click(await screen.findByTestId('mention-file-main.ts'));

    fireEvent.change(input, { target: { value: 'look at this file' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(submitted).toHaveLength(1));
    expect(submitted[0].op.items).toEqual([
      { type: 'text', text: 'look at this file' },
      { type: 'file', path: 'src/main.ts' },
    ]);
    // 提交后附件清空。
    expect(screen.queryByTestId('attachment-bar')).toBeNull();
  });
});
