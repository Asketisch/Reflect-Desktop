/**
 * TitleBar —— Main 区域顶部标题栏。
 *
 * 左：sidebar 折叠按钮 + 当前 view 标题 + session 状态(model @ provider / waiting)。
 * 右：inspector 折叠按钮 + permission mode。
 */
import { PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen } from 'lucide-react';
import { useLocation } from '@tanstack/react-router';
import { Icon, IconButton, Tooltip, Badge } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import s from './TitleBar.module.css';

/** 路径 → 标题映射。 */
const TITLES: Record<string, string> = {
  '/': 'Chat',
  '/home': 'Home',
  '/chat': 'Chat',
  '/sessions': 'Threads',
  '/files': 'Files',
  '/git': 'Git',
  '/terminal': 'Terminal',
  '/skills': 'Skills & Tools',
  '/workspaces': 'Workspaces',
  '/models': 'Models',
  '/plan': 'Plan Mode',
  '/prompts': 'Prompts',
  '/notifications': 'Notifications',
  '/settings': 'Settings',
  '/about': 'About',
  '/update': 'Updates',
  '/debug': 'Debug',
  '/apps': 'Apps',
  '/collaboration': 'Collaboration',
  '/mobile': 'Mobile',
  '/dictation': 'Dictation',
  '/design-system': 'Design System',
};

function titleFor(pathname: string): string {
  if (TITLES[pathname]) return TITLES[pathname];
  if (pathname.startsWith('/chat')) return 'Chat';
  return 'Reflect';
}

export function TitleBar({
  sidebarOpen,
  onToggleSidebar,
  inspectorOpen,
  onToggleInspector,
}: {
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  inspectorOpen: boolean;
  onToggleInspector: () => void;
}) {
  const location = useLocation();
  const session = useAgentStore((s) => s.session);
  const permissionMode = useAgentStore((s) => s.permissionMode);

  return (
    <header className={s.bar}>
      <div className={s.left}>
        <Tooltip label={sidebarOpen ? 'Hide sidebar' : 'Show sidebar'} side="bottom">
          <IconButton label={sidebarOpen ? 'Hide sidebar' : 'Show sidebar'} onClick={onToggleSidebar}>
            <Icon icon={sidebarOpen ? PanelLeftClose : PanelLeftOpen} size={16} />
          </IconButton>
        </Tooltip>
        <h1 className={s.title}>{titleFor(location.pathname)}</h1>
        {/* session 状态（保留 "session: (waiting...)" 文案契约，供 MessageList 测试断言） */}
        <span className={s.sessionStatus} data-testid="titlebar-session">
          session: {session ? `${session.model} @ ${session.provider}` : '(waiting...)'}
        </span>
      </div>
      <div className={s.right}>
        {permissionMode && (
          <Badge variant={permissionMode === 'auto' ? 'success' : permissionMode === 'plan' ? 'info' : 'warning'}>
            {permissionMode}
          </Badge>
        )}
        <Tooltip label={inspectorOpen ? 'Hide inspector' : 'Show inspector'} side="bottom">
          <IconButton label={inspectorOpen ? 'Hide inspector' : 'Show inspector'} onClick={onToggleInspector} variant={inspectorOpen ? 'active' : 'default'}>
            <Icon icon={inspectorOpen ? PanelRightClose : PanelRightOpen} size={16} />
          </IconButton>
        </Tooltip>
      </div>
    </header>
  );
}
