/**
 * Vitest — ToolCell (B7-05).
 */
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ToolCell } from './ToolCells';

describe('ToolCell', () => {
  it('renders header with tool name + summary', () => {
    render(<ToolCell toolName="shell" argsSummary='{"command":"ls"}' status="done" />);
    const cell = screen.getByTestId('tool-cell-shell');
    expect(cell).toBeDefined();
    expect(cell.querySelector('[class*="summary"]')?.textContent).toBe('$ ls');
  });

  it('expands args on click', () => {
    render(<ToolCell toolName="shell" argsSummary='{"command":"ls"}' status="done" />);
    const cell = screen.getByTestId('tool-cell-shell');
    expect(cell.querySelector('[data-testid="tool-args-shell"]')).toBeNull();
    fireEvent.click(cell.querySelector('button')!);
    expect(cell.querySelector('[data-testid="tool-args-shell"]')).not.toBeNull();
  });

  it('uses file icon for read_file', () => {
    const { container } = render(
      <ToolCell toolName="read_file" argsSummary='{"path":"/etc/passwd"}' status="done" />,
    );
    expect(container.querySelector('[data-tool="read_file"]')).toBeDefined();
    const cell = screen.getByTestId('tool-cell-read_file');
    expect(cell.querySelector('[class*="summary"]')?.textContent).toBe('/etc/passwd');
  });

  it('handles unparseable args gracefully', () => {
    render(<ToolCell toolName="shell" argsSummary="not-json" status="error" />);
    const cell = screen.getByTestId('tool-cell-shell');
    // 无法解析时，摘要回退为工具名
    expect(cell.querySelector('[class*="summary"]')?.textContent).toBe('shell');
    fireEvent.click(cell.querySelector('button')!);
    expect(screen.getByTestId('tool-args-shell').textContent).toBe('not-json');
  });

  it('falls back to generic wrench for unknown tools', () => {
    const { container } = render(
      <ToolCell toolName="my_custom_tool" argsSummary="{}" status="running" />,
    );
    expect(container.querySelector('[data-tool="my_custom_tool"]')).toBeDefined();
  });
});