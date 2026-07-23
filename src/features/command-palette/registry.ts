/**
 * CommandPalette registry (B10-01).
 *
 * 每个 PaletteItem 是一个可执行的命令(导航到路由 / 触发 slash /
 * 触发主题切换 / 触发回调)。`kind` 决定默认分组,搜索按 label /
 * keywords 模糊匹配。
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

export type PaletteItemKind =
  | 'navigation'
  | 'session'
  | 'slash'
  | 'theme'
  | 'system';

export interface PaletteItem {
  id: string;
  label: string;
  /** Optional one-line description. */
  hint?: string;
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
    { id: 'nav.home',        label: 'Go to Home',        kind: 'navigation', icon: Home,         to: '/home',        weight: 10 },
    { id: 'nav.chat',        label: 'Go to Chat',        kind: 'navigation', icon: MessagesSquare, to: '/chat',        weight: 10 },
    { id: 'nav.sessions',    label: 'Go to Sessions',    kind: 'navigation', icon: ListChecks,   to: '/sessions',    weight: 10 },
    { id: 'nav.settings',    label: 'Open Settings',     kind: 'navigation', icon: Settings,     to: '/settings',    weight: 10 },
    { id: 'nav.files',       label: 'Open Files',        kind: 'navigation', icon: FolderOpen,   to: '/files',       weight: 10 },
    { id: 'nav.models',      label: 'Open Models',       kind: 'navigation', icon: Cpu,          to: '/models',      weight: 10 },
    { id: 'nav.skills',      label: 'Open Skills',       kind: 'navigation', icon: Sparkles,     to: '/skills',      weight: 10 },
    { id: 'nav.workspaces',  label: 'Open Workspaces',   kind: 'navigation', icon: Folder,       to: '/workspaces',  weight: 10 },
    { id: 'nav.git',         label: 'Open Git',          kind: 'navigation', icon: GitBranch,    to: '/git',         weight: 10 },
    { id: 'nav.terminal',    label: 'Open Terminal',     kind: 'navigation', icon: TerminalIcon, to: '/terminal',    weight: 10 },
    { id: 'nav.plan',        label: 'Open Plan',         kind: 'navigation', icon: ListChecks,   to: '/plan',        weight: 10 },
    { id: 'nav.prompts',     label: 'Open Prompts',      kind: 'navigation', icon: BookOpen,     to: '/prompts',     weight: 10 },
    { id: 'nav.about',       label: 'Open About',        kind: 'navigation', icon: Info,         to: '/about',       weight: 10 },
    { id: 'nav.update',      label: 'Open Update',       kind: 'navigation', icon: Download,     to: '/update',      weight: 10 },
    { id: 'nav.notifications', label: 'Open Notifications', kind: 'navigation', icon: Bell,      to: '/notifications', weight: 10 },
    { id: 'nav.debug',       label: 'Open Debug',        kind: 'navigation', icon: Bug,          to: '/debug',       weight: 10 },
    { id: 'nav.apps',        label: 'Open Apps',         kind: 'navigation', icon: AppWindow,    to: '/apps',        weight: 10 },
    { id: 'nav.collab',      label: 'Open Collaboration', kind: 'navigation', icon: Users,        to: '/collaboration', weight: 10 },
    { id: 'nav.dictation',   label: 'Open Dictation',    kind: 'navigation', icon: Mic,          to: '/dictation',   weight: 10 },
    { id: 'nav.mobile',      label: 'Open Mobile',       kind: 'navigation', icon: Smartphone,   to: '/mobile',      weight: 10 },
    { id: 'nav.design',      label: 'Open Design System', kind: 'navigation', icon: Layers,      to: '/design-system', weight: 10 },

    // ---- Session actions ----
    { id: 'sess.new',    label: 'New session',         hint: 'Start a fresh chat',  kind: 'session', icon: Plus,    run: newSession, weight: 5 },
    { id: 'sess.clear',  label: 'Clear all sessions',  hint: 'Delete every session', kind: 'session', icon: Trash2,  run: clearAllSessions, weight: 5 },
    { id: 'sess.export', label: 'Export current session', hint: 'Write to ~/.reflect/exports/', kind: 'session', icon: Download, run: exportActive, weight: 5 },
    { id: 'cfg.save',    label: 'Save config',         hint: 'Write ~/.reflect/config.toml',  kind: 'system',  icon: Save,    run: saveConfig, weight: 5 },

    // ---- Slash actions ----
    { id: 'slash.compact',   label: 'Compact context',   slash: '/compact',   kind: 'slash', icon: RefreshCw, keywords: ['compress', 'summarize'], weight: 4 },
    { id: 'slash.interrupt', label: 'Interrupt turn',    slash: '/interrupt', kind: 'slash', icon: RefreshCw, keywords: ['stop', 'cancel'], weight: 4 },
    { id: 'slash.clear',     label: 'Clear screen',      slash: '/clear',     kind: 'slash', icon: Trash2,    weight: 4 },
    { id: 'slash.help',      label: 'Show slash help',   slash: '/help',      kind: 'slash', icon: BookOpen,  weight: 4 },
    { id: 'slash.rename',    label: 'Rename session',    slash: '/rename ',   kind: 'slash', icon: Wrench,    keywords: ['title', 'label'], weight: 4 },
    { id: 'slash.export',    label: 'Export session',    slash: '/export',    kind: 'slash', icon: Download,  weight: 4 },

    // ---- Theme ----
    { id: 'theme.cycle',     label: 'Cycle theme (dark → light → system)', kind: 'theme', icon: Palette, run: cycleTheme, weight: 3 },
    { id: 'theme.dark',      label: 'Switch to dark theme',  kind: 'theme', icon: Moon,    run: () => setTheme('dark'), weight: 3 },
    { id: 'theme.light',     label: 'Switch to light theme', kind: 'theme', icon: Sun,     run: () => setTheme('light'), weight: 3 },
    { id: 'theme.system',    label: 'Follow system theme',   kind: 'theme', icon: Monitor, run: () => setTheme('system'), weight: 3 },
    { id: 'theme.now',       label: `Currently: ${resolvedTheme} theme`, kind: 'theme', icon: resolvedTheme === 'dark' ? Moon : Sun, run: cycleTheme, weight: 9 },
  ];
}