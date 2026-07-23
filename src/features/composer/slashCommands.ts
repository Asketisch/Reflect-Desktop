/**
 * M1.5 Slash 命令静态列表 —— 镜像 `crates/reflect-tui/src/slash.rs` 的命令。
 *
 * Tier A (toolbar 显示) 9 个 + 剩余弹 popup。 M2.x 把 active command
 * 接到对应 Tauri command / API。
 */
export interface SlashCmd {
  name: string;
  aliases?: string[];
  category: 'theme' | 'mode' | 'model' | 'session' | 'memory' | 'tasks' | 'permissions' | 'skills' | 'shortcuts' | 'help' | 'system';
  summary: string;
  toolbared?: boolean;
}

export const SLASH_COMMANDS: SlashCmd[] = [
  { name: 'theme', category: 'theme', summary: '切换主题 (light/dark/system)', toolbared: true },
  { name: 'vim', category: 'mode', summary: '切换 vim 模式', toolbared: true },
  { name: 'effort', category: 'mode', summary: '设置 reasoning effort', toolbared: true },
  { name: 'compact', category: 'session', summary: '触发上下文压缩', toolbared: true },
  { name: 'mode', category: 'permissions', summary: '切换权限模式', toolbared: true },
  { name: 'model', category: 'model', summary: '切换当前 model', toolbared: true },
  { name: 'provider', aliases: ['prov'], category: 'model', summary: '切换 provider', toolbared: true },
  { name: 'status', category: 'system', summary: '查看状态 / 上下文用量', toolbared: true },
  { name: 'help', aliases: ['?'], category: 'help', summary: '显示所有 slash 命令', toolbared: true },

  { name: 'init', category: 'system', summary: '初始化当前目录' },
  { name: 'clear', category: 'session', summary: '清屏(对话继续)' },
  { name: 'interrupt', category: 'session', summary: '中断当前 turn' },
  { name: 'resume', category: 'session', summary: '恢复上一会话' },
  { name: 'rename', category: 'session', summary: '重命名当前会话' },
  { name: 'export', category: 'session', summary: '导出当前会话 (markdown/jsonl)' },
  { name: 'share', category: 'session', summary: '分享会话链接' },
  { name: 'session', aliases: ['sessions'], category: 'session', summary: '会话管理子命令' },
  { name: 'diff', category: 'system', summary: '查看 diff' },
  { name: 'files', category: 'system', summary: '列出最近文件' },
  { name: 'branch', category: 'system', summary: '分支管理' },
  { name: 'commit', category: 'system', summary: '提交当前 changes' },
  { name: 'commit-push-pr', category: 'system', summary: '提交 + push + PR' },
  { name: 'review', category: 'system', summary: '触发 review' },
  { name: 'ultrareview', category: 'system', summary: '深度 review' },
  { name: 'cost', category: 'system', summary: 'Cost tracker' },
  { name: 'usage', category: 'system', summary: 'Token 用量' },
  { name: 'copy', category: 'shortcuts', summary: '复制当前 turn' },
  { name: 'editor', category: 'shortcuts', summary: '外部编辑器' },
  { name: 'stats', category: 'system', summary: '查看会话统计' },
  { name: 'insights', category: 'system', summary: '查看 AI insights' },
  { name: 'context', category: 'system', summary: '查看 context 用量详情' },
  { name: 'ctx_viz', aliases: ['ctxviz'], category: 'system', summary: '可视化 context' },
  { name: 'color', category: 'theme', summary: '色彩主题' },
  { name: 'sandbox-toggle', category: 'permissions', summary: '切换 sandbox' },
  { name: 'memory', category: 'memory', summary: '记忆管理 (ls/show/edit/add/remove)' },
  { name: 'hooks', category: 'system', summary: 'Hook 管理' },
  { name: 'tasks', category: 'tasks', summary: '任务管理 (ls/show/clear)' },
  { name: 'permissions', category: 'permissions', summary: '权限规则 (ls/allow/deny)' },
  { name: 'skills', category: 'skills', summary: '技能管理' },
  { name: 'plan', category: 'mode', summary: '进入 Plan 模式' },
  { name: 'exit-plan', aliases: ['exitplan'], category: 'mode', summary: '退出 Plan 模式' },
  { name: 'keybindings', aliases: ['keys', 'bindings'], category: 'shortcuts', summary: '查看键盘绑定' },
  { name: 'statusline', aliases: ['status-line'], category: 'shortcuts', summary: '状态栏设置' },
  { name: 'output-style', category: 'shortcuts', summary: '输出样式' },
  { name: 'mcp', category: 'system', summary: 'MCP server 管理' },
  { name: 'plugin', category: 'system', summary: '插件管理' },
  { name: 'ide', category: 'system', summary: '启动 IDE 集成' },
  { name: 'desktop', category: 'system', summary: '桌面模式' },
  { name: 'mobile', category: 'system', summary: '移动模式' },
  { name: 'exit', aliases: ['quit'], category: 'system', summary: '退出应用' },
];

export const TIER_A_TOOLBAR = SLASH_COMMANDS.filter((c) => c.toolbared);
