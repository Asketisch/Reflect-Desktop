/**
 * CommandPalette registry (B10-01).
 *
 * 每个 PaletteItem 是一个可执行的命令(导航到路由 / 触发 slash /
 * 触发主题切换 / 触发回调)。`kind` 决定默认分组,搜索按 label /
 * keywords 模糊匹配。
 *
 * **i18n**:`label` / `hint` 改为 i18n key,渲染时通过 `t(labelKey)` / `t(hintKey)` 拿到。
 */
import type { ComponentType } from 'react';
import {
  Home,
  MessagesSquare,
  Settings,
  FolderOpen,
  Cpu,
  Sparkles,
  Folder,
  GitBranch,
  Terminal as TerminalIcon,
  ListChecks,
  BookOpen,
  Bell,
  Wrench,
  Layers,
  Smartphone,
  Mic,
  AppWindow,
  Users,
  Bug,
  Palette,
  Info,
  RefreshCw,
  Moon,
  Sun,
  Monitor,
  Plus,
  Trash2,
  Download,
  Save,
} from 'lucide-react';
import type { LocaleKey } from '@/utils/i18n';

export type PaletteItemKind =
  | 'navigation'
  | 'session'
  | 'slash'
  | 'theme'
  | 'system';

export interface PaletteItem {
  id: string;
  labelKey: LocaleKey;
  /** Optional one-line description. */
  hintKey?: LocaleKey;
  kind: PaletteItemKind;
  icon: ComponentType;
  /** Free-text keywords for fuzzy search. */
  keywords?: string[];
  /** Activation — exactly one of these is set. */
  to?: string;
  slash?: string;
  run?: () => void;
  /** Lower = earlier in the list. */
  weight?: number;
}

/** Build a snapshot of available palette items at the moment of opening. */
export interface BuildPaletteArgs {
  navigate: (to: string) => void;
  cycleTheme: () => void;
  setTheme: (mode: 'light' | 'dark' | 'system') => void;
  resolvedTheme: 'light' | 'dark';
  newSession: () => void;
  clearAllSessions: () => void;
  exportActive: () => void;
  saveConfig: () => void;
}

export function buildPaletteItems(args: BuildPaletteArgs): PaletteItem[] {
  const {
    cycleTheme, setTheme, resolvedTheme, newSession, clearAllSessions, exportActive, saveConfig,
  } = args;
  return [
    // ---- Navigation ----
    { id: 'nav.home',        labelKey: 'palette.item.goHome',      kind: 'navigation', icon: Home,         to: '/home',        weight: 10 },
    { id: 'nav.chat',        labelKey: 'palette.item.goChat',      kind: 'navigation', icon: MessagesSquare, to: '/chat',        weight: 10 },
    { id: 'nav.sessions',    labelKey: 'palette.item.goSessions',  kind: 'navigation', icon: ListChecks,   to: '/sessions',    weight: 10 },
    { id: 'nav.settings',    labelKey: 'palette.item.goSettings',  kind: 'navigation', icon: Settings,     to: '/settings',    weight: 10 },
    { id: 'nav.files',       labelKey: 'palette.item.goFiles',     kind: 'navigation', icon: FolderOpen,   to: '/files',       weight: 10 },
    { id: 'nav.models',      labelKey: 'palette.item.goModels',    kind: 'navigation', icon: Cpu,          to: '/models',      weight: 10 },
    { id: 'nav.skills',      labelKey: 'palette.item.goSkills',    kind: 'navigation', icon: Sparkles,     to: '/skills',      weight: 10 },
    { id: 'nav.workspaces',  labelKey: 'palette.item.goWorkspaces', kind: 'navigation', icon: Folder,       to: '/workspaces',  weight: 10 },
    { id: 'nav.git',         labelKey: 'palette.item.goGit',       kind: 'navigation', icon: GitBranch,    to: '/git',         weight: 10 },
    { id: 'nav.terminal',    labelKey: 'palette.item.goTerminal',  kind: 'navigation', icon: TerminalIcon, to: '/terminal',    weight: 10 },
    { id: 'nav.plan',        labelKey: 'palette.item.goPlan',      kind: 'navigation', icon: ListChecks,   to: '/plan',        weight: 10 },
    { id: 'nav.prompts',     labelKey: 'palette.item.goPrompts',   kind: 'navigation', icon: BookOpen,     to: '/prompts',     weight: 10 },
    { id: 'nav.about',       labelKey: 'palette.item.goAbout',     kind: 'navigation', icon: Info,         to: '/about',       weight: 10 },
    { id: 'nav.update',      labelKey: 'palette.item.goUpdate',    kind: 'navigation', icon: Download,     to: '/update',      weight: 10 },
    { id: 'nav.notifications', labelKey: 'palette.item.goNotifications', kind: 'navigation', icon: Bell,      to: '/notifications', weight: 10 },
    { id: 'nav.debug',       labelKey: 'palette.item.goDebug',     kind: 'navigation', icon: Bug,          to: '/debug',       weight: 10 },
    { id: 'nav.apps',        labelKey: 'palette.item.goApps',      kind: 'navigation', icon: AppWindow,    to: '/apps',        weight: 10 },
    { id: 'nav.collab',      labelKey: 'palette.item.goCollab',    kind: 'navigation', icon: Users,        to: '/collaboration', weight: 10 },
    { id: 'nav.dictation',   labelKey: 'palette.item.goDictation', kind: 'navigation', icon: Mic,          to: '/dictation',   weight: 10 },
    { id: 'nav.mobile',      labelKey: 'palette.item.goMobile',    kind: 'navigation', icon: Smartphone,   to: '/mobile',      weight: 10 },
    { id: 'nav.design',      labelKey: 'palette.item.goDesign',    kind: 'navigation', icon: Layers,      to: '/design-system', weight: 10 },

    // ---- Session actions ----
    { id: 'sess.new',    labelKey: 'palette.item.newSession',    hintKey: 'palette.item.newSessionHint',   kind: 'session', icon: Plus,    run: newSession, weight: 5 },
    { id: 'sess.clear',  labelKey: 'palette.item.clearSessions', hintKey: 'palette.item.clearSessionsHint', kind: 'session', icon: Trash2,  run: clearAllSessions, weight: 5 },
    { id: 'sess.export', labelKey: 'palette.item.exportActive',  hintKey: 'palette.item.exportActiveHint', kind: 'session', icon: Download, run: exportActive, weight: 5 },
    { id: 'cfg.save',    labelKey: 'palette.item.saveConfig',    hintKey: 'palette.item.saveConfigHint',   kind: 'system',  icon: Save,    run: saveConfig, weight: 5 },

    // ---- Slash actions ----
    { id: 'slash.compact',   labelKey: 'palette.item.runCompact',     slash: '/compact',   kind: 'slash', icon: RefreshCw, keywords: ['compress', 'summarize'], weight: 4 },
    { id: 'slash.interrupt', labelKey: 'palette.item.runInterrupt',   slash: '/interrupt', kind: 'slash', icon: RefreshCw, keywords: ['stop', 'cancel'], weight: 4 },
    { id: 'slash.clear',     labelKey: 'palette.item.runClear',       slash: '/clear',     kind: 'slash', icon: Trash2,    weight: 4 },
    { id: 'slash.help',      labelKey: 'palette.item.runHelp',        slash: '/help',      kind: 'slash', icon: BookOpen,  weight: 4 },
    { id: 'slash.rename',    labelKey: 'palette.item.runRename',      slash: '/rename ',   kind: 'slash', icon: Wrench,    keywords: ['title', 'label'], weight: 4 },
    { id: 'slash.export',    labelKey: 'palette.item.runExport',      slash: '/export',    kind: 'slash', icon: Download,  weight: 4 },
    { id: 'slash.new',       labelKey: 'palette.item.runNewSession',  slash: '/new',       kind: 'slash', icon: Plus,      weight: 4 },

    // ---- Theme ----
    { id: 'theme.cycle',     labelKey: 'palette.item.toggleTheme',    kind: 'theme', icon: Palette, run: cycleTheme, weight: 3 },
    { id: 'theme.dark',      labelKey: 'palette.item.setThemeDark',   kind: 'theme', icon: Moon,    run: () => setTheme('dark'), weight: 3 },
    { id: 'theme.light',     labelKey: 'palette.item.setThemeLight',  kind: 'theme', icon: Sun,     run: () => setTheme('light'), weight: 3 },
    { id: 'theme.system',    labelKey: 'palette.item.setThemeSystem', kind: 'theme', icon: Monitor, run: () => setTheme('system'), weight: 3 },
    { id: 'theme.now',       labelKey: 'palette.item.themeCurrent',   kind: 'theme', icon: resolvedTheme === 'dark' ? Moon : Sun, run: cycleTheme, weight: 9 },
  ];
}
