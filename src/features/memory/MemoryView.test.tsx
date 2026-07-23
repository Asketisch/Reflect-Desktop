/**
 * Vitest — MemoryView (B11-01).
 *
 * Smoke tests: render empty state + verify filters/form are present.
 * Real CRUD is exercised against a Tauri backend in integration tests.
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { MemoryView } from './MemoryView';
import { createTestQueryClient } from '@/test/setup.tsx';

// Mock backend calls
vi.mock('@/utils/commands', () => {
  const actual = vi.importActual<typeof import('@/utils/commands')>('@/utils/commands');
  return {
    ...actual,
    reflect_list_memory: async () => [],
    reflect_add_memory: async () => {},
    reflect_remove_memory: async () => {},
  };
});

function wrap(node: React.ReactNode) {
  return <QueryClientProvider client={createTestQueryClient()}>{node}</QueryClientProvider>;
}

describe('MemoryView', () => {
  beforeEach(() => {
    cleanup();
  });

  it('renders the page title', () => {
    render(wrap(<MemoryView />));
    expect(screen.getByText('Memory')).toBeDefined();
  });

  it('renders scope filter buttons', () => {
    render(wrap(<MemoryView />));
    expect(screen.getByTestId('memory-filter-all')).toBeDefined();
    expect(screen.getByTestId('memory-filter-global')).toBeDefined();
    expect(screen.getByTestId('memory-filter-project')).toBeDefined();
    expect(screen.getByTestId('memory-filter-session')).toBeDefined();
  });

  it('renders the add button', () => {
    render(wrap(<MemoryView />));
    expect(screen.getByTestId('memory-add-btn')).toBeDefined();
  });

  it('shows add form when toggled', () => {
    render(wrap(<MemoryView />));
    const btn = screen.getByTestId('memory-add-btn');
    fireEvent.click(btn);
    expect(screen.getByTestId('memory-new-key')).toBeDefined();
    expect(screen.getByTestId('memory-new-value')).toBeDefined();
  });

  it('displays empty state when no memory entries', async () => {
    render(wrap(<MemoryView />));
    // useQuery fires async — wait for the empty text to appear
    const el = await screen.findByText(/No memory entries/i, {}, { timeout: 3000 });
    expect(el).toBeDefined();
  });
});