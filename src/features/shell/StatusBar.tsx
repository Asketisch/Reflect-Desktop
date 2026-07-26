/**
 * StatusBar —— IDE 风底部状态栏。
 *
 * 左：model @ provider · permission · effort · workspace。
 * 右：MCP/LSP 状态点 · 错误计数 · 主题切换。
 *
 * **数据优先级**（避免「no model」误报）：
 *   1. `useAgentStore.session` —— 由 `session_configured` 事件填充，最权威。
 *   2. fallback 到 `reflect_agent_status` 查询结果（启动期 / 降级模式 / 事件未到时）。
 *   3. 都没有时才显示「no model」。
 *
 * 替代旧的 widgets/StatusBar（Topbar/BottomBar），移除所有 "M1.7 scaffold" 字样。
 */
import { useState, useEffect } from 'react';
import { Sun, Moon, Monitor, AlertCircle, MessagesSquare } from 'lucide-react';
import { Icon, IconButton, Tooltip } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import { useQuery } from '@tanstack/react-query';
import { reflect_agent_status, reflect_list_sessions } from '@/utils/commands';
import { getTheme, setTheme, getResolvedTheme, subscribeTheme, type ThemeMode } from '@/utils/theme';
import { useI18n } from '@/utils/i18n';
import s from './StatusBar.module.css';

export function StatusBar() {
  const session = useAgentStore((st) => st.session);
  const permissionMode = useAgentStore((st) => st.permissionMode);
  const cyclePermission = useAgentStore((st) => st.cyclePermissionMode);
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const lastError = useAgentStore((st) => st.lastError);
  const clearError = useAgentStore((st) => st.clearError);
  const { t, tp } = useI18n();

  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });
  const sessionsQ = useQuery({
    queryKey: ['sessions'],
    queryFn: () => reflect_list_sessions(),
    staleTime: 60_000,
  });
  const sessionCount = sessionsQ.data?.length ?? 0;

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

  // 模型显示：优先 store.session（事件填充），fallback 到 status 查询。
  // 三态：configured（绿）/ degraded 黄 / unknown 红。
  const status = statusQ.data;
  const modelLabel = session
    ? `${session.model} @ ${session.provider}`
    : status && status.has_model
      ? status.model
      : null;
  const modelKind: 'ok' | 'warn' | 'error' = session
    ? 'ok'
    : status && status.has_model
      ? 'ok'
      : status && status.degraded_reason
        ? 'warn'
        : 'warn';
  const modelTooltip = status?.degraded_reason ?? undefined;
  const workspaceLabel = status?.workspace ? basename(status.workspace) : null;

  return (
    <footer className={s.bar}>
      <div className={s.group}>
        {modelLabel ? (
          <span className={s.item} title={modelTooltip}>
            <span className={s.dot} data-kind={modelKind} />
            {modelLabel}
          </span>
        ) : (
          <Tooltip label={modelTooltip ?? t('shell.noModelTooltip')} side="top">
            <span className={s.item}>
              <span className={s.dot} data-kind={modelKind} />
              {t('shell.noModel')}
            </span>
          </Tooltip>
        )}
        <button className={s.btn} onClick={() => cyclePermission()} title={t('shell.cyclePermission')}>
          {permissionMode}
        </button>
        {workspaceLabel && (
          <span className={s.itemMuted} title={status?.workspace}>
            {workspaceLabel}
          </span>
        )}
      </div>

      <div className={s.group}>
        {lastError && (
          <Tooltip label={lastError} side="top">
            <button className={s.item} onClick={clearError} data-kind="error">
              <Icon icon={AlertCircle} size={12} />
              {t('shell.errorButton')}
            </button>
          </Tooltip>
        )}
        {(mcpFailed > 0 || lspFailed > 0) && (
          <Tooltip label={t('shell.mcpLspFailed', { mcp: mcpFailed, lsp: lspFailed })} side="top">
            <span className={s.item} data-kind="error">
              {t('shell.failedCount', { count: mcpFailed + lspFailed })}
            </span>
          </Tooltip>
        )}
        {mcpServers.length > 0 && (
          <span className={s.itemMuted}>
            {t('shell.mcpCount', { count: mcpServers.length })}
          </span>
        )}
        <Tooltip label={tp('shell.sessionCount', sessionCount, { count: sessionCount })} side="top">
          <span className={s.itemMuted} data-testid="statusbar-session-count">
            <Icon icon={MessagesSquare} size={11} /> {sessionCount}
          </span>
        </Tooltip>
        <Tooltip label={t('shell.themeCurrent', { mode: mode })} side="top">
          <IconButton
            label={t('shell.themeCurrent', { mode: mode })}
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
