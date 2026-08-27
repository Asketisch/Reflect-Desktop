/**
 * M1.5 Slash 命令静态列表 —— 镜像 `crates/reflect-tui/src/slash.rs` 的命令。
 *
 * Tier A (toolbar 显示) 9 个 + 剩余弹 popup。 M2.x 把 active command
 * 接到对应 Tauri command / API。
 *
 * summary 是 i18n key,渲染时通过 t(summaryKey) 拿到当前 locale 文本。
 */
export interface SlashCmd {
  name: string;
  aliases?: string[];
  category: 'theme' | 'mode' | 'model' | 'session' | 'memory' | 'tasks' | 'permissions' | 'skills' | 'shortcuts' | 'help' | 'system';
  summaryKey: string;
  toolbared?: boolean;
}

export const SLASH_COMMANDS: SlashCmd[] = [
  { name: 'theme', category: 'theme', summaryKey: 'slash.theme.desc', toolbared: true },
  { name: 'vim', category: 'mode', summaryKey: 'slash.vim.desc', toolbared: true },
  { name: 'effort', category: 'mode', summaryKey: 'slash.effort.desc', toolbared: true },
  { name: 'compact', category: 'session', summaryKey: 'slash.compact.desc', toolbared: true },
  { name: 'mode', category: 'permissions', summaryKey: 'slash.mode.desc', toolbared: true },
  { name: 'model', category: 'model', summaryKey: 'slash.model.desc', toolbared: true },
  { name: 'provider', aliases: ['prov'], category: 'model', summaryKey: 'slash.provider.desc', toolbared: true },
  { name: 'status', category: 'system', summaryKey: 'slash.status.desc', toolbared: true },
  { name: 'help', aliases: ['?'], category: 'help', summaryKey: 'slash.help.desc', toolbared: true },

  { name: 'init', category: 'system', summaryKey: 'slash.init.desc' },
  { name: 'clear', category: 'session', summaryKey: 'slash.clear.desc' },
  { name: 'interrupt', category: 'session', summaryKey: 'slash.interrupt.desc' },
  { name: 'resume', category: 'session', summaryKey: 'slash.resume.desc' },
  { name: 'rename', category: 'session', summaryKey: 'slash.rename.desc' },
  { name: 'export', category: 'session', summaryKey: 'slash.export.desc' },
  { name: 'share', category: 'session', summaryKey: 'slash.share.desc' },
  { name: 'session', aliases: ['sessions'], category: 'session', summaryKey: 'slash.session.desc' },
  { name: 'diff', category: 'system', summaryKey: 'slash.diff.desc' },
  { name: 'files', category: 'system', summaryKey: 'slash.files.desc' },
  { name: 'branch', category: 'system', summaryKey: 'slash.branch.desc' },
  { name: 'commit', category: 'system', summaryKey: 'slash.commit.desc' },
  { name: 'commit-push-pr', category: 'system', summaryKey: 'slash.commitPushPr.desc' },
  { name: 'review', category: 'system', summaryKey: 'slash.review.desc' },
  { name: 'ultrareview', category: 'system', summaryKey: 'slash.ultrareview.desc' },
  { name: 'cost', category: 'system', summaryKey: 'slash.cost.desc' },
  { name: 'usage', category: 'system', summaryKey: 'slash.usage.desc' },
  { name: 'copy', category: 'shortcuts', summaryKey: 'slash.copy.desc' },
  { name: 'editor', category: 'shortcuts', summaryKey: 'slash.editor.desc' },
  { name: 'stats', category: 'system', summaryKey: 'slash.stats.desc' },
  { name: 'insights', category: 'system', summaryKey: 'slash.insights.desc' },
  { name: 'context', category: 'system', summaryKey: 'slash.context.desc' },
  { name: 'ctx_viz', aliases: ['ctxviz'], category: 'system', summaryKey: 'slash.ctxViz.desc' },
  { name: 'color', category: 'theme', summaryKey: 'slash.color.desc' },
  { name: 'sandbox-toggle', category: 'permissions', summaryKey: 'slash.sandboxToggle.desc' },
  { name: 'memory', category: 'memory', summaryKey: 'slash.memory.desc' },
  { name: 'hooks', category: 'system', summaryKey: 'slash.hooks.desc' },
  { name: 'tasks', category: 'tasks', summaryKey: 'slash.tasks.desc' },
  { name: 'permissions', category: 'permissions', summaryKey: 'slash.permissions.desc' },
  { name: 'skills', category: 'skills', summaryKey: 'slash.skills.desc' },
  { name: 'plan', category: 'mode', summaryKey: 'slash.enterPlan.desc' },
  { name: 'exit-plan', aliases: ['exitplan'], category: 'mode', summaryKey: 'slash.exitPlan.desc' },
  { name: 'goal', category: 'mode', summaryKey: 'slash.goal.desc' },
  { name: 'keybindings', aliases: ['keys', 'bindings'], category: 'shortcuts', summaryKey: 'slash.keybindings.desc' },
  { name: 'statusline', aliases: ['status-line'], category: 'shortcuts', summaryKey: 'slash.statusline.desc' },
  { name: 'output-style', category: 'shortcuts', summaryKey: 'slash.outputStyle.desc' },
  { name: 'mcp', category: 'system', summaryKey: 'slash.mcp.desc' },
  { name: 'plugin', category: 'system', summaryKey: 'slash.plugin.desc' },
  { name: 'ide', category: 'system', summaryKey: 'slash.ide.desc' },
  { name: 'desktop', category: 'system', summaryKey: 'slash.desktop.desc' },
  { name: 'mobile', category: 'system', summaryKey: 'slash.mobile.desc' },
  { name: 'exit', aliases: ['quit'], category: 'system', summaryKey: 'slash.exit.desc' },
];

export const TIER_A_TOOLBAR = SLASH_COMMANDS.filter((c) => c.toolbared);
