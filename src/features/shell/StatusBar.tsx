/**
 * StatusBar —— IDE 风底部状态栏。
 *
 * 左：model @ provider · permission · effort · workspace。
 * 右：MCP/LSP 状态点 · 错误计数 · 主题切换。
 *
 * 替代旧的 widgets/StatusBar（Topbar/BottomBar），移除所有 "M1.7 scaffold" 字样。
 */
import { useState, useEffect } from 'react';
import { Sun, Moon, Monitor, AlertCircle } from 'lucide-react';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import { useQuery } from '@tanstack/react-query';
import { reflect_agent_status } from '@/utils/tauri';
import { getTheme, setTheme, getResolvedTheme, subscribeTheme, type ThemeMode } from '@/utils/theme';
import s from './StatusBar.module.css';

export function StatusBar() {
  const session = useAgentStore((st) => st.session);
  const permissionMode = useAgentStore((st) => st.permissionMode);
  const cyclePermission = useAgentStore((st) => st.cyclePermissionMode);
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const lastError = useAgentStore((st) => st.lastError);
  const clearError = useAgentStore((st) => st.clearError);

  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  // 主题：resolved 态驱动图标，mode 驱动循环。
  const [resolved, setResolved] = useState<'light' | 'dark'>(() => getResolvedTheme());
  const [mode, setMode] = useState<ThemeMode>(() => getTheme());
  useEffect(() => subscribeTheme(setResolved), []);

  const cycleTheme = () => {
    const next: ThemeMode = mode === 'dark' ? 'light' : mode === 'light' ? 'system' : 'dark';
    setTheme(next);
    setMode(next);
  };

  const mcpFailed = mcpServers.filter((m) => m.status === 'failed').length;
  const lspFailed = lspServers.filter((l) => l.status === 'failed').length;

  return (
    <footer className={s.bar}>
      <div className={s.group}>
        {session ? (
          <span className={s.item}>
            <span className={s.dot} data-kind="ok" />
            {session.model} @ {session.provider}
          </span>
        ) : (
          <span className={s.item}>
            <span className={s.dot} data-kind="warn" />
            no model
          </span>
        )}
        <button className={s.btn} onClick={() => cyclePermission()} title="Cycle permission mode">
          {permissionMode}
        </button>
        {statusQ.data?.workspace && (
          <span className={s.itemMuted} title={statusQ.data.workspace}>
            {basename(statusQ.data.workspace)}
          </span>
        )}
      </div>

      <div className={s.group}>
        {lastError && (
          <Tooltip label={lastError} side="top">
            <button className={s.item} onClick={clearError} data-kind="error">
              <Icon icon={AlertCircle} size={12} />
              error
            </button>
          </Tooltip>
        )}
        {(mcpFailed > 0 || lspFailed > 0) && (
          <Tooltip label={`${mcpFailed} MCP / ${lspFailed} LSP failed`} side="top">
            <span className={s.item} data-kind="error">
              {mcpFailed + lspFailed} failed
            </span>
          </Tooltip>
        )}
        {mcpServers.length > 0 && (
          <span className={s.itemMuted}>
            MCP {mcpServers.length}
          </span>
        )}
        <Tooltip label={`Theme: ${mode}`} side="top">
          <IconButton
            label={`Theme: ${mode}`}
            size="sm"
            onClick={cycleTheme}
            className={s.themeBtn}
          >
            <Icon icon={mode === 'system' ? Monitor : resolved === 'dark' ? Moon : Sun} size={13} />
          </IconButton>
        </Tooltip>
      </div>
    </footer>
  );
}

function basename(p: string): string {
  const parts = p.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] || p;
}
