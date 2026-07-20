/**
 * TitleBar —— 贯通全宽的顶栏（macOS Overlay 标题栏）。
 *
 * 关键设计：
 *   - 根元素挂 `data-tauri-drag-region` —— 整条顶栏可拖动窗口。
 *   - 高度 = `--titlebar-height` token（40px，macOS 标准）。
 *   - 左侧 `--traffic-light-gutter` (80px) padding 为红绿灯按钮让位。
 *   - 内容：sidebar toggle + 当前 view 标题 + session 状态 / 右侧 permission + inspector toggle。
 *
 * **session 状态文案**：保留 `session: (waiting…)` 字面契约（测试断言依赖），
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
    // data-tauri-drag-region：让整条顶栏可拖动窗口。
    // 内部的 button/a/input 由 base.css 的 `[data-tauri-drag-region] button { no-drag }` 自动豁免，
    // 保证 sidebar/inspector toggle 仍可点击。
    <header className={s.bar} data-tauri-drag-region data-testid="titlebar">
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
