/**
 * AgentsView —— Phase 1 item 3 agent profile management.
 *
 * Lists agent definitions from `reflect_list_agent_defs`, supports
 * create / edit / delete. The editor form covers name / description /
 * model / system_prompt / tools / disallowed_tools / spawnable / readonly /
 * max_turns / memory scopes.
 *
 * Backend contract: `src/utils/commands/agents.ts` ↔
 * `src-tauri/src/commands/agents.rs` ↔ `vendor/reflect-agent-def`.
 * Storage: `~/.reflect/agents/<name>.md`, shared with TUI/CLI.
 */
import { Bot, Plus, Pencil, Trash2 } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Card, EmptyState, Icon, Spinner } from '@/features/design-system';
import {
  joinCsv,
  parseCsv,
  useAgentsController,
  type ReflectAgentDef,
} from './useAgentsController';
import { AgentEditor } from './AgentEditor';
import s from './AgentsView.module.css';

export function AgentsView() {
  const ctrl = useAgentsController();

  return (
    <PageShell
      icon={Bot}
      title="Agents"
      subtitle="Agent profiles — model, system prompt, tools. Stored at ~/.reflect/agents/*.md."
      width="lg"
      actions={
        <button
          type="button"
          className={s.addBtn}
          onClick={ctrl.beginCreate}
          disabled={ctrl.isEditing}
          data-testid="agents-add-btn"
        >
          <Icon icon={Plus} size={12} /> New
        </button>
      }
    >
      {/* Editor */}
      {ctrl.isEditing && ctrl.draft && (
        <AgentEditor
          draft={ctrl.draft}
          onPatch={ctrl.patchDraft}
          onSave={() => void ctrl.save()}
          onCancel={ctrl.cancelEdit}
        />
      )}

      {/* List */}
      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Bot} />}
            title="Failed to load agents"
            description="Check the agent backend and try again."
          />
        </Card>
      ) : ctrl.defs.length === 0 && !ctrl.isEditing ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Bot} />}
            title="No agents defined"
            description="Click “New” to create your first agent profile."
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="agents-list">
          {ctrl.defs.map((def) => (
            <AgentRow
              key={def.name}
              def={def}
              onEdit={() => ctrl.beginEdit(def)}
              onRemove={() => void ctrl.remove(def)}
            />
          ))}
        </div>
      )}
    </PageShell>
  );
}

function AgentRow({
  def,
  onEdit,
  onRemove,
}: {
  def: ReflectAgentDef;
  onEdit: () => void;
  onRemove: () => void;
}) {
  return (
    <div className={s.row} data-testid={`agent-row-${def.name}`}>
      <div className={s.rowMain}>
        <code className={s.rowName}>{def.name}</code>
        <span className={s.rowDesc}>{def.description}</span>
        {def.model && <Badge variant="accent">{def.model}</Badge>}
        {def.readonly && <Badge variant="neutral">readonly</Badge>}
        {def.spawnable && <Badge variant="success">spawnable</Badge>}
      </div>
      <div className={s.rowSide}>
        <div className={s.rowActions}>
          <button
            type="button"
            className={s.actionBtn}
            onClick={onEdit}
            title="Edit"
            data-testid={`agent-edit-${def.name}`}
          >
            <Icon icon={Pencil} size={12} />
          </button>
          <button
            type="button"
            className={`${s.actionBtn} ${s.actionBtnDanger}`}
            onClick={onRemove}
            title="Delete"
            data-testid={`agent-delete-${def.name}`}
          >
            <Icon icon={Trash2} size={12} />
          </button>
        </div>
      </div>
    </div>
  );
}

// Re-export for test convenience.
export { joinCsv, parseCsv };
