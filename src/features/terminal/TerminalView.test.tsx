/**
 * Vitest — TerminalView (B8-01).
 *
 * Smoke tests: render empty state + verify the form/input are present.
 * Streaming + kill are exercised by manual integration tests against a real
 * Tauri runtime.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TerminalView } from './TerminalView';

describe('TerminalView', () => {
  beforeEach(() => {
    // nothing — tests are presentational; backend calls are mocked via setup.tsx.
  });

  it('renders the empty state when no sessions', () => {
    render(<TerminalView />);
    expect(screen.getByText(/No active sessions/i)).toBeDefined();
  });

  it('renders the input form', () => {
    render(<TerminalView />);
    expect(screen.getByTestId('terminal-input')).toBeDefined();
  });

  it('renders the header with Terminal label', () => {
    render(<TerminalView />);
    expect(screen.getByText('Terminal')).toBeDefined();
  });

  it('renders the placeholder hint', () => {
    render(<TerminalView />);
    expect(screen.getByText(/Type a shell command below/i)).toBeDefined();
  });
});