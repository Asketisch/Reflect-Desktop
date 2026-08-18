/**
 * RemoteView —— Phase 2 条目 2（Tailscale + iOS 守护进程）。
 *
 * 渲染：
 *   - iOS 连接设置卡片（host / port / auth_token 表单）
 *   - 桌面端传输状态（当前始终为 "disconnected — driver TBD"）
 *   - Tailscale 检测卡片（已安装 / 运行中 / 建议的 host）
 *   - 桌面守护进程启动/停止占位符的 "Not implemented" 提示
 *
 * 后端契约：`src/utils/commands/remote.ts` ↔
 * `src-tauri/src/commands/remote.rs` ↔ `app-core::tailscale` +
 * `state::RemoteConfig`。
 */
import { useState } from 'react';
import { Wifi, Save, X, RefreshCw, Loader2, Info } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Card, EmptyState, Icon, Spinner } from '@/features/design-system';
import {
  useRemoteController,
  type RemoteConfigDraft,
} from './useRemoteController';
import s from './RemoteView.module.css';

export function RemoteView() {
  const ctrl = useRemoteController();
  const [copyOk, setCopyOk] = useState(false);

  const copyPreview = async () => {
    const text = ctrl.previewCmd;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopyOk(true);
      setTimeout(() => setCopyOk(false), 1500);
    } catch {
      /* 剪贴板不可用 */
    }
  };

  return (
    <PageShell
      icon={Wifi}
      title="Remote"
      subtitle="Tailscale + iOS companion app. Configure the desktop daemon endpoint your iOS app connects to."
      width="lg"
      actions={
        <button
          type="button"
          className={s.refreshBtn}
          onClick={ctrl.refetch}
          data-testid="remote-refresh"
        >
          <Icon icon={RefreshCw} size={12} /> Refresh
        </button>
      }
    >
      {/* iOS connection setup */}
      <Card level="outlined" padding="md" className={s.section} data-testid="remote-config-card">
        <header className={s.sectionHeader}>
          <div>
            <strong>iOS connection</strong>
            <div className={s.sectionHint}>
              Endpoint + shared token your iOS app uses to reach this desktop.
            </div>
          </div>
          <div className={s.sectionActions}>
            <Badge variant={ctrl.config?.is_ready ? 'success' : 'neutral'}>
              {ctrl.config?.is_ready ? 'ready' : 'not configured'}
            </Badge>
            <button
              type="button"
              className={s.editBtn}
              onClick={ctrl.toggleShowForm}
              data-testid="remote-edit-btn"
            >
              {ctrl.showForm ? <Icon icon={X} size={12} /> : <Save size={12} />}
              {' '}
              {ctrl.showForm ? 'Cancel' : ctrl.config?.is_ready ? 'Edit' : 'Configure'}
            </button>
          </div>
        </header>

        {ctrl.config && !ctrl.showForm && (
          <dl className={s.configList} data-testid="remote-config-display">
            <div className={s.configRow}>
              <dt>Endpoint</dt>
              <dd>{ctrl.config.endpoint || '(empty)'}</dd>
            </div>
            <div className={s.configRow}>
              <dt>Host</dt>
              <dd>{ctrl.config.host || '(empty)'}</dd>
            </div>
            <div className={s.configRow}>
              <dt>Port</dt>
              <dd>{ctrl.config.port}</dd>
            </div>
            <div className={s.configRow}>
              <dt>Auth token</dt>
              <dd>{ctrl.config.auth_token ? `${ctrl.config.auth_token.slice(0, 4)}…` : '(not set)'}</dd>
            </div>
            <div className={s.configRow}>
              <dt>Auto-connect</dt>
              <dd>{ctrl.config.auto_connect ? 'yes' : 'no'}</dd>
            </div>
          </dl>
        )}

        {ctrl.showForm && (
          <form
            className={s.form}
            data-testid="remote-config-form"
            onSubmit={(e) => {
              e.preventDefault();
              void ctrl.save();
            }}
          >
            <label className={s.field}>
              Host
              <input
                className={s.input}
                value={ctrl.draft.host}
                onChange={(e) => ctrl.patchDraft({ host: e.target.value } as Partial<RemoteConfigDraft>)}
                placeholder="e.g. node.tail.net or 100.64.0.1"
                data-testid="remote-form-host"
              />
            </label>
            <label className={s.field}>
              Port
              <input
                className={s.input}
                type="number"
                value={ctrl.draft.port}
                onChange={(e) =>
                  ctrl.patchDraft({ port: Number(e.target.value) || 4732 })
                }
                data-testid="remote-form-port"
              />
            </label>
            <label className={s.field}>
              Auth token
              <input
                className={s.input}
                type="password"
                value={ctrl.draft.authToken}
                onChange={(e) => ctrl.patchDraft({ authToken: e.target.value })}
                placeholder="shared bearer token"
                data-testid="remote-form-token"
              />
            </label>
            <label className={s.checkboxField}>
              <input
                type="checkbox"
                checked={ctrl.draft.autoConnect}
                onChange={(e) => ctrl.patchDraft({ autoConnect: e.target.checked })}
                data-testid="remote-form-auto-connect"
              />{' '}
              Auto-connect on launch (driver TBD)
            </label>
            <div className={s.formActions}>
              <button
                type="button"
                className={s.cancelBtn}
                onClick={ctrl.toggleShowForm}
                data-testid="remote-form-cancel"
              >
                Cancel
              </button>
              <button
                type="submit"
                className={s.submitBtn}
                disabled={ctrl.isMutating}
                data-testid="remote-form-submit"
              >
                {ctrl.isMutating ? <Icon icon={Loader2} size={12} /> : <Save size={12} />}
                {' '}Save
              </button>
            </div>
          </form>
        )}
      </Card>

      {/* Tailscale */}
      <Card level="outlined" padding="md" className={s.section} data-testid="remote-tailscale-card">
        <header className={s.sectionHeader}>
          <div>
            <strong>Tailscale</strong>
            <div className={s.sectionHint}>
              Detects the local daemon + suggests the MagicDNS hostname for iOS.
            </div>
          </div>
          <Badge variant={!ctrl.tailscale ? 'neutral' : ctrl.tailscale.running ? 'success' : 'warning'}>
            {ctrl.tailscale?.installed ? (ctrl.tailscale.running ? 'running' : 'installed') : 'not installed'}
          </Badge>
        </header>

        {ctrl.tailscale && (
          <dl className={s.configList}>
            <div className={s.configRow}>
              <dt>Suggested host</dt>
              <dd>{ctrl.tailscale.suggested_remote_host || '(no DNS or IP)'}</dd>
            </div>
            {ctrl.tailscale.dns_name && (
              <div className={s.configRow}>
                <dt>DNS</dt>
                <dd>{ctrl.tailscale.dns_name}</dd>
              </div>
            )}
            {ctrl.tailscale.tailnet_name && (
              <div className={s.configRow}>
                <dt>Tailnet</dt>
                <dd>{ctrl.tailscale.tailnet_name}</dd>
              </div>
            )}
            {ctrl.tailscale.ipv4.length > 0 && (
              <div className={s.configRow}>
                <dt>IPv4</dt>
                <dd>{ctrl.tailscale.ipv4.join(', ')}</dd>
              </div>
            )}
            {ctrl.tailscale.message && (
              <div className={s.configRow}>
                <dt>Message</dt>
                <dd>{ctrl.tailscale.message}</dd>
              </div>
            )}
          </dl>
        )}
      </Card>

      {/* Daemon hint */}
      <Card level="outlined" padding="md" className={s.section} data-testid="remote-daemon-card">
        <header className={s.sectionHeader}>
          <div>
            <strong>Desktop daemon hint</strong>
            <div className={s.sectionHint}>
              Run this on the desktop to expose the JSON-RPC endpoint for iOS.
            </div>
          </div>
          <button
            type="button"
            className={s.copyBtn}
            onClick={copyPreview}
            disabled={!ctrl.previewCmd}
            data-testid="remote-copy-preview"
          >
            {copyOk ? 'Copied' : 'Copy'}
          </button>
        </header>
        {ctrl.previewCmd ? (
          <pre className={s.codeBlock} data-testid="remote-preview-text">{ctrl.previewCmd}</pre>
        ) : ctrl.loading ? (
          <div className={s.loading}>
            <Spinner size={16} />
          </div>
        ) : (
          <EmptyState
            icon={<Icon icon={Info} />}
            title="Preview unavailable"
            description="The Tailscale daemon helper could not render a preview."
          />
        )}
      </Card>

      {/* Transport status */}
      <Card level="outlined" padding="md" className={s.section} data-testid="remote-status-card">
        <header className={s.sectionHeader}>
          <div>
            <strong>Transport status</strong>
            <div className={s.sectionHint}>
              Live connection state to the configured iOS / 远端 daemon.
            </div>
          </div>
          <Badge variant={ctrl.status?.state === 'connected' ? 'success' : 'neutral'}>
            {ctrl.status?.state ?? 'unknown'}
          </Badge>
        </header>
        {ctrl.status?.message && (
          <p className={s.statusMsg}>{ctrl.status.message}</p>
        )}
      </Card>
    </PageShell>
  );
}