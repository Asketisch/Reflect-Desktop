/**
 * TitleBar —— Main 区域顶部标题栏。
 *
 * 左：sidebar 折叠按钮 + 当前 view 标题 + session 状态(model @ provider / waiting)。
 * 右：inspector 折叠按钮 + permission mode。
 *
 * **session 状态文案**：保留 `session: (waiting...)` 字面契约（测试断言依赖），
 * 但在 `session_configured` 事件未到时，从 `reflect_agent_status` 查询拿 model 名，
 * 避免开局永远显示「waiting」。
 */
import { PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen } from 'lucide-react';
import { useLocation } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import { Icon, IconButton, Tooltip, Badge } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_agent_status } from '@/utils/tauri';
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

  // 后端诊断查询 —— session_configured 事件未到时的 fallback。
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  const sessionLabel = session
    ? `${session.model} @ ${session.provider}`
    : statusQ.data?.has_model
      ? statusQ.data.model
      : '(waiting…)';

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
          session: {sessionLabel}
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
