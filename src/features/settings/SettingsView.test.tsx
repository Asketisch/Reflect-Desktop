/**
 * Vitest — SettingsView 组件测试。
 *
 * 验证:
 * 1. 渲染 settings 表单
 * 2. 切换 permission mode
 * 3. 调用 reflect_set_permission_mode on save
 * 4. 显示 saved 状态
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { SettingsView } from '@/features/settings/SettingsView';
import { mockInvoke, resetMockInvoke, createTestQueryClient } from '@/test/setup.tsx';

function renderSettingsView() {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <SettingsView />
    </QueryClientProvider>
  );
}

describe('SettingsView', () => {
  beforeEach(() => {
    resetMockInvoke();
    mockInvoke('reflect_set_permission_mode', async () => {});
  });

  it('renders settings form', () => {
    renderSettingsView();
    
    expect(screen.getByText('Settings')).toBeDefined();
    expect(screen.getByText('Display')).toBeDefined();
    expect(screen.getByText('Editor')).toBeDefined();
    expect(screen.getByText('Provider')).toBeDefined();
    expect(screen.getByText('Permissions')).toBeDefined();
  });

  it('shows permission mode buttons (Auto/Ask/Deny)', () => {
    renderSettingsView();
    
    // Use getAllByText since React StrictMode may render elements twice
    const autoElements = screen.getAllByText('Auto');
    expect(autoElements.length).toBeGreaterThanOrEqual(1);
    const askElements = screen.getAllByText('Ask');
    expect(askElements.length).toBeGreaterThanOrEqual(1);
    const denyElements = screen.getAllByText('Deny');
    expect(denyElements.length).toBeGreaterThanOrEqual(1);
  });

  it('calls reflect_set_permission_mode on save', async () => {
    let modeArg: string | null = null;
    // Mock handler receives (cmd, args) per setup.tsx mock pattern
    mockInvoke('reflect_set_permission_mode', async (_cmd: string, args: unknown) => {
      modeArg = (args as { mode: string }).mode;
    });

    renderSettingsView();

    // Click the Deny permission button (not the select option)
    const denyElements = screen.getAllByText('Deny');
    const denyButton = denyElements.find(el => el.tagName === 'BUTTON');
    expect(denyButton).toBeDefined();
    fireEvent.click(denyButton!);

    // Click Save Settings
    const saveElements = screen.getAllByText('Save Settings');
    const saveButton = saveElements.find(el => el.tagName === 'BUTTON');
    expect(saveButton).toBeDefined();
    fireEvent.click(saveButton!);

    await waitFor(() => expect(modeArg).toBe('Deny'), { timeout: 2000 });
  });

  it('shows saved state after save', async () => {
    renderSettingsView();

    // Click Save Settings
    const saveButtons = screen.getAllByText('Save Settings');
    fireEvent.click(saveButtons[0]);

    // setSaved(true) is synchronous, button text changes immediately
    // Use getAllByText since "Saved ✓" may appear in multiple places
    const savedElements = screen.queryAllByText('Saved ✓');
    expect(savedElements.length).toBeGreaterThan(0);
  });
});
