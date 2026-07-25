import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { I18nProvider } from '@/utils/i18n';
import { createTestQueryClient } from '@/test/setup';
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
    fireEvent.keyDown(input, {
      key: 'Enter',
      metaKey: true,
      isComposing: true,
    });
    expect(input.value).toBe('composing');
  });
});
