/**
 * ActivityBar —— IDE 风左侧竖条导航。
 *
 * 顶部：核心导航（Chat / Files / Git / Skills / ...）。
 * 底部：Settings / About。
 * active 态从当前 location 派生。
 */
import { MessageSquare, FolderGit2, Wrench, GitBranch, Bell, Settings, Info, Terminal, Home } from 'lucide-react';
import type { ComponentType } from 'react';
import { useRouter, useLocation } from '@tanstack/react-router';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import s from './ActivityBar.module.css';

interface NavItem {
  to: string;
  label: string;
  icon: ComponentType;
  /** 路由前缀匹配（用于 active 判定，如 /chat 匹配 /chat/$id）。 */
  matchPrefix?: string;
}

const PRIMARY: NavItem[] = [
  { to: '/home', label: 'Home', icon: Home },
  { to: '/', label: 'Chat', icon: MessageSquare, matchPrefix: '/chat' },
  { to: '/files', label: 'Files', icon: FolderGit2 },
  { to: '/git', label: 'Git', icon: GitBranch },
  { to: '/terminal', label: 'Terminal', icon: Terminal },
  { to: '/skills', label: 'Skills & Tools', icon: Wrench },
  { to: '/notifications', label: 'Notifications', icon: Bell },
];

const SECONDARY: NavItem[] = [
  { to: '/settings', label: 'Settings', icon: Settings },
  { to: '/about', label: 'About', icon: Info },
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

  return (
    <nav className={s.bar} aria-label="Main navigation">
      <div className={s.logo} aria-hidden="true" title="Reflect Desktop">
        R
      </div>
      <div className={s.group}>
        {PRIMARY.map((item) => {
          const active = isActive(pathname, item);
          return (
            <Tooltip key={item.to} label={item.label} side="right">
              <IconButton
                label={item.label}
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
          return (
            <Tooltip key={item.to} label={item.label} side="right">
              <IconButton
                label={item.label}
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
