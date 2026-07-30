/**
 * AgentEditor — create/edit form for an AgentDefinition.
 *
 * Presentational; state lives in `useAgentsController.draft` + `patchDraft`.
 * Covers all `AgentDefinition` fields except `mcp_collections` (reserved v1).
 */
import { Save, X } from 'lucide-react';
import { Card, Icon, Textarea, Input } from '@/features/design-system';
import type { ReflectAgentDef, ReflectMemoryScope } from '@/utils/commands';
import { joinCsv, parseCsv } from './useAgentsController';
import s from './AgentsView.module.css';

const MEMORY_SCOPES: ReflectMemoryScope[] = ['project', 'user', 'session'];

export interface AgentEditorProps {
  draft: ReflectAgentDef;
  onPatch: (patch: Partial<ReflectAgentDef>) => void;
  onSave: () => void;
  onCancel: () => void;
}

export function AgentEditor({ draft, onPatch, onSave, onCancel }: AgentEditorProps) {
  const isNew = !draft.name;
  return (
    <Card level="outlined" padding="md" className={s.editor} data-testid="agent-editor">
      <div className={s.editorHeader}>
        <strong>{isNew ? 'New agent' : `Edit “${draft.name}”`}</strong>
        <div className={s.editorActions}>
          <button
            type="button"
            className={s.actionBtn}
            onClick={onCancel}
            data-testid="agent-editor-cancel"
          >
            <Icon icon={X} size={12} /> Cancel
          </button>
          <button
            type="button"
            className={s.saveBtn}
            onClick={onSave}
            data-testid="agent-editor-save"
          >
            <Icon icon={Save} size={12} /> Save
          </button>
        </div>
      </div>

      <form
        className={s.editorForm}
        onSubmit={(e) => {
          e.preventDefault();
          onSave();
        }}
      >
        <label className={s.fieldLabel}>
          Name <span className={s.required}>*</span>
          <Input
            value={draft.name}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => onPatch({ name: e.target.value })}
            placeholder="e.g. code-reviewer"
            data-testid="agent-editor-name"
            disabled={!isNew}
          />
        </label>

        <label className={s.fieldLabel}>
          Description <span className={s.required}>*</span>
          <Input
            value={draft.description}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              onPatch({ description: e.target.value })
            }
            placeholder="Short human-readable description"
            data-testid="agent-editor-description"
          />
        </label>

        <label className={s.fieldLabel}>
          Model
          <Input
            value={draft.model ?? ''}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              onPatch({ model: e.target.value || null })
            }
            placeholder="inherit (default), or provider/model"
            data-testid="agent-editor-model"
          />
        </label>

        <label className={s.fieldLabel}>
          Tools (comma-separated; empty = all builtins)
          <Input
            value={joinCsv(draft.tools)}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              onPatch({ tools: parseCsv(e.target.value) })
            }
            placeholder="read, grep, glob"
            data-testid="agent-editor-tools"
          />
        </label>

        <label className={s.fieldLabel}>
          Disallowed tools (comma-separated)
          <Input
            value={joinCsv(draft.disallowed_tools)}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              onPatch({ disallowed_tools: parseCsv(e.target.value) })
            }
            placeholder="bash, edit"
            data-testid="agent-editor-disallowed"
          />
        </label>

        <div className={s.fieldRow}>
          <label className={s.checkbox}>
            <input
              type="checkbox"
              checked={draft.spawnable}
              onChange={(e) => onPatch({ spawnable: e.target.checked })}
              data-testid="agent-editor-spawnable"
            />{' '}
            Spawnable (other agents may invoke)
          </label>
          <label className={s.checkbox}>
            <input
              type="checkbox"
              checked={draft.readonly}
              onChange={(e) => onPatch({ readonly: e.target.checked })}
              data-testid="agent-editor-readonly"
            />{' '}
            Readonly (no write/edit tools)
          </label>
        </div>

        <div className={s.fieldRow}>
          <label className={s.fieldLabel}>
            Max turns
            <Input
              type="number"
              value={draft.max_turns ?? ''}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                onPatch({
                  max_turns: e.target.value ? Number(e.target.value) : null,
                })
              }
              placeholder="unset"
              data-testid="agent-editor-max-turns"
            />
          </label>
        </div>

        <div className={s.fieldLabel}>
          Memory scopes
          <div className={s.checkboxRow}>
            {MEMORY_SCOPES.map((scope) => (
              <label key={scope} className={s.checkbox}>
                <input
                  type="checkbox"
                  checked={draft.memory.includes(scope)}
                  onChange={(e) => {
                    const next = e.target.checked
                      ? [...draft.memory, scope]
                      : draft.memory.filter((s) => s !== scope);
                    onPatch({ memory: next });
                  }}
                  data-testid={`agent-editor-memory-${scope}`}
                />{' '}
                {scope}
              </label>
            ))}
          </div>
        </div>

        <label className={s.fieldLabel}>
          System prompt (markdown body)
          <Textarea
            value={draft.system_prompt}
            onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
              onPatch({ system_prompt: e.target.value })
            }
            placeholder="You are a strict code reviewer..."
            rows={8}
            data-testid="agent-editor-system-prompt"
          />
        </label>

        {/* Hidden submit so Enter triggers save */}
        <button type="submit" style={{ display: 'none' }} />
      </form>
    </Card>
  );
}
