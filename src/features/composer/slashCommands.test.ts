/**
 * Vitest — SLASH_COMMANDS 数据完整性测试。
 */
import { describe, it, expect } from 'vitest';
import { SLASH_COMMANDS, TIER_A_TOOLBAR } from '@/features/composer/slashCommands';

describe('SLASH_COMMANDS', () => {
  it('has unique command names', () => {
    const names = SLASH_COMMANDS.map((c) => c.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it('each command has category', () => {
    for (const c of SLASH_COMMANDS) {
      expect(c.category).toBeTruthy();
    }
  });

  it('each command has summaryKey', () => {
    for (const c of SLASH_COMMANDS) {
      expect(c.summaryKey.length).toBeGreaterThan(0);
    }
  });

  it('includes 7 toolbar commands (Tier A)', () => {
    expect(TIER_A_TOOLBAR.length).toBe(7);
    const toolbarNames = TIER_A_TOOLBAR.map((c) => c.name);
    // 纯 TUI 镜像 stub（theme/vim）已下架。
    expect(toolbarNames).not.toContain('theme');
    expect(toolbarNames).not.toContain('vim');
    expect(toolbarNames).toContain('effort');
    expect(toolbarNames).toContain('compact');
    expect(toolbarNames).toContain('mode');
    expect(toolbarNames).toContain('model');
  });

  it('does not list TUI-only stub commands', () => {
    const names = SLASH_COMMANDS.map((c) => c.name);
    expect(names).not.toContain('theme');
    expect(names).not.toContain('vim');
  });

  it('includes aliases where defined', () => {
    const help = SLASH_COMMANDS.find((c) => c.name === 'help');
    expect(help?.aliases).toContain('?');
    const mode = SLASH_COMMANDS.find((c) => c.name === 'mode');
    expect(mode?.aliases).toBeUndefined();
  });

  it('aliases are also unique', () => {
    const allAliases: string[] = [];
    for (const c of SLASH_COMMANDS) {
      if (c.aliases) allAliases.push(...c.aliases);
    }
    expect(new Set(allAliases).size).toBe(allAliases.length);
  });
});