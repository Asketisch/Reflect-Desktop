/**
 * SideChannelView —— Phase 2 item 1 user-driven concurrent agent panel.
 *
 * Lists side-channels (`reflect_list_side_channels`), supports
 * start / cancel. The driver task that actually executes a side-channel
 * (running its prompt as a `Submission::user_input` against the agent loop)
 * is a follow-up; this view shows the registry state and lets the user
 * create / cancel entries today.
 *
 * Backend contract: `src/utils/commands/side_channel.ts` ↔
 * `src-tauri/src/commands/side_channel.rs` ↔
 * `reflect-app-core::side_channel::SideChannelRegistry`.
 */
import { Plus, X, GitBranch, XCircle, Loader2 } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Card, EmptyState, Icon, Spinner } from '@/features/design-system';
import { useSideChannelController } from './useSideChannelController';
import type { ReflectSideChannelInfo, ReflectSideChannelStatus } from '@/utils/commands';
import s from './SideChannelView.module.css';

const STATUS_VARIANT: Record<ReflectSideChannelStatus, 'neutral' | 'accent' | 'success' | 'danger'> = {
  running: 'accent',
  done: 'success',
  cancelled: 'neutral',
  error: 'danger',
};

export function SideChannelView() {
  const ctrl = useSideChannelController();

  return (
    <PageShell
      icon={GitBranch}
      title="Side-channels"
      subtitle="Concurrent user-driven agent runs. Each side-channel has an independent cancel token — main Cmd+C does not stop them."
      width="lg"
      actions={
        <Badge variant={ctrl.runningCount > 0 ? 'accent' : 'neutral'}>
          {ctrl.runningCount} running
        </Badge>
      }
    >
      {/* Start form */}
      {ctrl.showForm && (
        <form
          className={s.startForm}
          data-testid="side-channel-start-form"
          onSubmit={(e) => {
            e.preventDefault();
            void ctrl.start();
          }}
        >
          <input
            className={s.formInput}
            placeholder="Agent name (e.g. default, reviewer)"
            value={ctrl.agentName}
            onChange={(e) => ctrl.setAgentName(e.target.value)}
            data-testid="side-channel-start-name"
          />
          <input
            className={s.formInput}
            placeholder="Prompt (required)"
            value={ctrl.prompt}
            onChange={(e) => ctrl.setPrompt(e.target.value)}
            data-testid="side-channel-start-prompt"
          />
          <div className={s.formActions}>
            <button
              type="button"
              className={s.cancelBtn}
              onClick={ctrl.toggleShowForm}
              data-testid="side-channel-start-cancel"
            >
              <Icon icon={X} size={12} /> Cancel
            </button>
            <button
              type="submit"
              className={s.submitBtn}
              disabled={ctrl.isMutating}
              data-testid="side-channel-start-submit"
            >
              {ctrl.isMutating ? <Icon icon={Loader2} size={12} /> : <Icon icon={Plus} size={12} />}
              Start
            </button>
          </div>
        </form>
      )}
      {!ctrl.showForm && (
        <div className={s.toolbar}>
          <button
            type="button"
            className={s.addBtn}
            onClick={ctrl.toggleShowForm}
            data-testid="side-channel-add-btn"
          >
            <Icon icon={Plus} size={12} /> Start a side-channel
          </button>
        </div>
      )}

      {/* Body */}
      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={GitBranch} />}
            title="Failed to load side-channels"
            description="Check the agent backend and try again."
          />
        </Card>
      ) : ctrl.channels.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={GitBranch} />}
            title="No side-channels"
            description="Click “Start a side-channel” above to spawn one."
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="side-channel-list">
          {ctrl.channels.map((c) => (
            <SideChannelRow
              key={c.id}
              channel={c}
              onCancel={() => void ctrl.cancel(c.id)}
            />
          ))}
        </div>
      )}
    </PageShell>
  );
}

function SideChannelRow({
  channel,
  onCancel,
}: {
  channel: ReflectSideChannelInfo;
  onCancel: () => void;
}) {
  const isRunning = channel.status === 'running';
  return (
    <div className={s.row} data-testid={`side-channel-row-${channel.id}`}>
      <div className={s.rowMain}>
        <code className={s.rowId}>{channel.id}</code>
        <span className={s.rowName}>{channel.agentName}</span>
        <span className={s.rowPrompt}>{channel.prompt}</span>
        {channel.durationMs != null && (
          <span className={s.rowDuration}>
            {(channel.durationMs / 1000).toFixed(1)}s
          </span>
        )}
      </div>
      <div className={s.rowSide}>
        <Badge variant={STATUS_VARIANT[channel.status]}>
          {isRunning && <Icon icon={Loader2} size={10} />} {channel.status}
        </Badge>
        {isRunning && (
          <button
            type="button"
            className={`${s.actionBtn} ${s.actionBtnDanger}`}
            onClick={onCancel}
            title="Cancel"
            data-testid={`side-channel-cancel-${channel.id}`}
          >
            <Icon icon={XCircle} size={12} /> Cancel
          </button>
        )}
      </div>
    </div>
  );
}
