/**
 * ActivityBar —— IDE 风左侧竖条导航。
 *
 * 顶部：核心导航（Chat / Files / Git / Skills / ...）。
 * 底部：Settings / About。
 * active 态从当前 location 派生。
 */
import { MessageSquare, FolderGit2, Wrench, GitBranch, Bell, Settings, Info, Terminal, Home, FolderKanban, Clock, Bot, Workflow, Wifi, Mic, BookOpen, Zap, Users, Image } from 'lucide-react';
import type { ComponentType } from 'react';
import { useRouter, useLocation } from '@tanstack/react-router';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { useI18n, type LocaleKey } from '@/utils/i18n';
import s from './ActivityBar.module.css';

interface NavItem {
  to: string;
  labelKey: LocaleKey;
  icon: ComponentType;
  /** 路由前缀匹配（用于 active 判定，如 /chat 匹配 /chat/$id）。 */
  matchPrefix?: string;
}

const PRIMARY: NavItem[] = [
  { to: '/home', labelKey: 'shell.nav.home', icon: Home },
  { to: '/', labelKey: 'shell.nav.chat', icon: MessageSquare, matchPrefix: '/chat' },
  { to: '/files', labelKey: 'shell.nav.files', icon: FolderGit2 },
  { to: '/git', labelKey: 'shell.nav.git', icon: GitBranch },
  { to: '/terminal', labelKey: 'shell.nav.terminal', icon: Terminal },
  { to: '/skills', labelKey: 'shell.nav.skills', icon: Wrench },
  { to: '/notifications', labelKey: 'shell.nav.notifications', icon: Bell },
  { to: '/tasks', labelKey: 'shell.nav.tasks', icon: FolderKanban },
  { to: '/schedule', labelKey: 'shell.nav.schedule', icon: Clock },
  { to: '/agents', labelKey: 'shell.nav.agents', icon: Bot },
  { to: '/side-channels', labelKey: 'shell.nav.sideChannels', icon: Workflow },
  { to: '/remote', labelKey: 'shell.nav.remote', icon: Wifi },
  { to: '/dictation', labelKey: 'shell.nav.dictation', icon: Mic },
  { to: '/kms', labelKey: 'shell.nav.kms', icon: BookOpen },
  { to: '/autopilot', labelKey: 'shell.nav.autopilot', icon: Zap },
  { to: '/squad', labelKey: 'shell.nav.squad', icon: Users },
  { to: '/media', labelKey: 'shell.nav.media', icon: Image },
];

const SECONDARY: NavItem[] = [
  { to: '/settings', labelKey: 'shell.nav.settings', icon: Settings },
  { to: '/about', labelKey: 'shell.nav.about', icon: Info },
];

function isActive(pathname: string, item: NavItem): boolean {
  if (item.to === '/') {
    // index 路由：仅正好根路径或 /chat/$id 时 active。
    return pathname === '/' || pathname === '/chat' || pathname.startsWith('/chat/');
  }
  const prefix = item.matchPrefix ?? item.to;
  return pathname === prefix || pathname.startsWith(prefix + '/');
}

export function ActivityBar() {
  const router = useRouter();
  const location = useLocation();
  const pathname = location.pathname;
  const { t } = useI18n();

  return (
    <nav className={s.bar} aria-label={t('shell.activityBar')}>
      <div className={s.logo} aria-hidden="true" title={t('shell.logoTitle')}>
        R
      </div>
      <div className={s.group}>
        {PRIMARY.map((item) => {
          const active = isActive(pathname, item);
          const label = t(item.labelKey);
          return (
            <Tooltip key={item.to} label={label} side="right">
              <IconButton
                label={label}
                variant={active ? 'active' : 'default'}
                size="lg"
                aria-current={active ? 'page' : undefined}
                onClick={() => router.navigate({ to: item.to })}
              >
                <Icon icon={item.icon} size={18} />
              </IconButton>
            </Tooltip>
          );
        })}
      </div>
      <div className={s.spacer} />
      <div className={s.group}>
        {SECONDARY.map((item) => {
          const active = isActive(pathname, item);
          const label = t(item.labelKey);
          return (
            <Tooltip key={item.to} label={label} side="right">
              <IconButton
                label={label}
                variant={active ? 'active' : 'default'}
                size="lg"
                aria-current={active ? 'page' : undefined}
                onClick={() => router.navigate({ to: item.to })}
              >
                <Icon icon={item.icon} size={18} />
              </IconButton>
            </Tooltip>
          );
        })}
      </div>
    </nav>
  );
}
