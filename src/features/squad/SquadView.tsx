/**
 * SquadView —— Squad 列表 + 详情 + 任务委派 (Phase 3 条目 11).
 *
 * Master-detail 布局:
 * - 左:所有 squad 列表(选中后高亮)+ 创建表单。
 * - 右:选中 squad 的成员表 + 该 squad 下的任务 + leader 委派按钮。
 */
import { useState } from 'react';
import { Plus, Users, Trash2, Zap, UserPlus, X } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import {
  Card,
  Icon,
  EmptyState,
  Spinner,
  Input,
  Textarea,
  Button,
  Badge,
} from '@/features/design-system';
import { useSquadController } from './useSquadController';
import type { ReflectSquadMember, ReflectTask } from '@/utils/commands';
import s from './SquadView.module.css';

export function SquadView() {
  const ctrl = useSquadController();

  return (
    <PageShell
      icon={Users}
      title="Squad"
      subtitle="Multi-agent teams with leader delegation."
      width="lg"
    >
      <div className={s.layout}>
        {/* ── Left: list + create form ──────────────────────────── */}
        <div className={s.col}>
          <div className={s.section}>
            <h2 className={s.sectionTitle}>Squads</h2>
            {ctrl.loading ? (
              <div className={s.loading}><Spinner size={20} /></div>
            ) : ctrl.squads.length === 0 ? (
              <Card level="flat" padding="none">
                <EmptyState
                  icon={<Icon icon={Users} />}
                  title="No squads yet"
                  description="Create one below to start coordinating multi-agent work."
                />
              </Card>
            ) : (
              <div className={s.list} data-testid="squad-list">
                {ctrl.squads.map((sq) => (
                  <button
                    key={sq.name}
                    type="button"
                    className={s.listItem}
                    data-active={ctrl.selectedName === sq.name || undefined}
                    onClick={() => ctrl.setSelectedName(sq.name)}
                    data-testid={`squad-${sq.name}`}
                  >
                    <div className={s.listName}>{sq.name}</div>
                    <div className={s.listMeta}>
                      {sq.members.length} member{sq.members.length === 1 ? '' : 's'}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className={s.section}>
            <h2 className={s.sectionTitle}>Create squad</h2>
            <Card level="outlined" padding="md">
              <div className={s.form}>
                <Input
                  placeholder="squad-name (a-z0-9_-)"
                  value={ctrl.draftName}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => ctrl.setDraftName(e.target.value)}
                  data-testid="squad-name"
                />
                <Textarea
                  placeholder="Description (optional)"
                  value={ctrl.draftDescription}
                  onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => ctrl.setDraftDescription(e.target.value)}
                />
                <MemberEditor
                  members={ctrl.draftMembers}
                  onChange={ctrl.setDraftMembers}
                  squadName={ctrl.draftName}
                />
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => void ctrl.create()}
                  disabled={!ctrl.draftName || ctrl.isMutating}
                  data-testid="squad-create"
                >
                  <Icon icon={Plus} size={12} /> Create
                </Button>
              </div>
            </Card>
          </div>
        </div>

        {/* ── Right: details + tasks + delegate ────────────────── */}
        <div className={s.col}>
          {ctrl.selectedSquad ? (
            <SquadDetail ctrl={ctrl} />
          ) : (
            <Card level="flat" padding="none">
              <EmptyState
                icon={<Icon icon={Users} />}
                title="Select a squad"
                description="Pick a squad from the list to view members and delegate tasks."
              />
            </Card>
          )}
        </div>
      </div>
    </PageShell>
  );
}

// ── SquadDetail (right pane) ───────────────────────────────────────

function SquadDetail(props: {
  ctrl: ReturnType<typeof useSquadController>;
}) {
  const { ctrl } = props;
  const squad = ctrl.selectedSquad!;
  return (
    <div className={s.detail}>
      <div className={s.detailHeader}>
        <h2 className={s.detailName}>{squad.name}</h2>
        <Badge variant="accent">leader: {squad.leaderActor.actorId}</Badge>
        <div className={s.spacer} />
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void ctrl.remove(squad.name)}
          data-testid="squad-delete"
        >
          <Icon icon={Trash2} size={12} /> Delete
        </Button>
      </div>

      {squad.description && <p className={s.desc}>{squad.description}</p>}

      {/* ── Members ────────────────────────────────────────────── */}
      <div className={s.section}>
        <h3 className={s.subsectionTitle}>Members ({squad.members.length})</h3>
        {squad.members.length === 0 ? (
          <p className={s.muted}>No additional members; the lead operates solo.</p>
        ) : (
          <div className={s.membersList} data-testid="squad-members">
            {squad.members.map((m) => (
              <Card key={m.actor.actorId} level="outlined" padding="sm">
                <div className={s.memberRow}>
                  <div>
                    <div className={s.memberRole}>{m.role}</div>
                    <div className={s.memberId}>{m.actor.actorId}</div>
                  </div>
                  {m.model && <Badge variant="neutral">{m.model}</Badge>}
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* ── Delegate / Tasks ──────────────────────────────────── */}
      <div className={s.section}>
        <div className={s.delegateHeader}>
          <h3 className={s.subsectionTitle}>Tasks in this squad</h3>
          <Button
            variant="primary"
            size="sm"
            onClick={() => void ctrl.delegateNext()}
            disabled={ctrl.isMutating}
            data-testid="squad-delegate"
          >
            <Icon icon={Zap} size={12} /> Delegate next
          </Button>
        </div>

        {ctrl.squadTasks.length === 0 ? (
          <p className={s.muted}>
            No tasks yet. Create one via the Tasks board (<code>list_id={squad.name}</code>).
          </p>
        ) : (
          <div className={s.tasksList} data-testid="squad-tasks">
            {ctrl.squadTasks.map((t) => (
              <SquadTaskRow
                key={t.id}
                task={t}
                members={squad.members}
                squadName={squad.name}
                onAssign={async (assignee) => {
                  await ctrl.assignTask(t.id, assignee);
                }}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function SquadTaskRow(props: {
  task: ReflectTask;
  members: ReflectSquadMember[];
  squadName: string;
  onAssign: (assignee: string | null) => Promise<void>;
}) {
  const { task, members, onAssign } = props;
  const [selected, setSelected] = useState(task.owner ?? '');
  const [busy, setBusy] = useState(false);
  return (
    <Card level="outlined" padding="sm">
      <div className={s.taskRow}>
        <div className={s.taskInfo}>
          <div className={s.taskTitle}>
            #{task.id} {task.subject}
          </div>
          <div className={s.taskMeta}>
            <Badge variant={task.status === 'completed' ? 'success' : task.status === 'in_progress' ? 'info' : 'neutral'}>
              {task.status.replace('_', ' ')}
            </Badge>
            {task.owner && <span> · owner: {task.owner}</span>}
          </div>
        </div>
        <div className={s.taskActions}>
          <select
            value={selected}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setSelected(e.target.value)}
            className={s.assignSelect}
            data-testid={`squad-assign-${task.id}`}
          >
            <option value="">— unassigned —</option>
            {members.map((m) => (
              <option key={m.actor.actorId} value={m.actor.actorId}>
                {m.role} ({m.actor.actorId})
              </option>
            ))}
          </select>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy || selected === (task.owner ?? '')}
            onClick={async () => {
              setBusy(true);
              try {
                await onAssign(selected || null);
              } finally {
                setBusy(false);
              }
            }}
            data-testid={`squad-assign-btn-${task.id}`}
          >
            <Icon icon={UserPlus} size={12} /> Assign
          </Button>
        </div>
      </div>
    </Card>
  );
}

// ── MemberEditor（用于创建表单）────────────────────────────────

function MemberEditor(props: {
  members: ReflectSquadMember[];
  onChange: (ms: ReflectSquadMember[]) => void;
  squadName: string;
}) {
  const { members, onChange, squadName } = props;
  function addRow() {
    onChange([
      ...members,
      {
        actor: {
          actorType: 'agent',
          actorId: squadName ? `member@${squadName}` : 'member@<squad>',
          kind: 'member',
          displayName: 'member',
          teamName: squadName || '',
        },
        role: 'member',
        model: null,
        systemPrompt: '',
        allowedTools: [],
      },
    ]);
  }
  function updateRole(idx: number, role: string) {
    const next = [...members];
    const clean = role.trim().toLowerCase() || 'member';
    next[idx] = {
      ...next[idx],
      role: clean,
      actor: {
        ...next[idx].actor,
        actorId: squadName ? `${clean}@${squadName}` : next[idx].actor.actorId,
        displayName: clean,
      },
    };
    onChange(next);
  }
  function remove(idx: number) {
    onChange(members.filter((_, i) => i !== idx));
  }
  return (
    <div className={s.memberEditor}>
      {members.map((m, i) => (
        <div key={i} className={s.memberEditorRow}>
          <Input
            placeholder="role (e.g. architect)"
            value={m.role}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateRole(i, e.target.value)}
            data-testid={`member-role-${i}`}
          />
          <Button variant="ghost" size="sm" onClick={() => remove(i)}>
            <Icon icon={X} size={12} />
          </Button>
        </div>
      ))}
      <Button variant="ghost" size="sm" onClick={addRow} data-testid="member-add">
        <Icon icon={Plus} size={12} /> Add member
      </Button>
    </div>
  );
}
