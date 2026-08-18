/**
 * AgentsView —— 阶段 1 任务 3 agent 配置文件管理。
 *
 * 列出 `reflect_list_agent_defs` 的 agent 定义，支持
 * 创建 / 编辑 / 删除。编辑器表单覆盖 name / description /
 * model / system_prompt / tools / disallowed_tools / spawnable / readonly /
 * max_turns / memory scopes。
 *
 * 后端契约：`src/utils/commands/agents.ts` ↔
 * `src-tauri/src/commands/agents.rs` ↔ `reflect-agent/crates/abilities/reflect-agent-def`。
 * 存储：`~/.reflect/agents/<name>.md`，与 TUI/CLI 共享。
 */
import { Bot, Plus, Pencil, Trash2 } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Card, EmptyState, Icon, Spinner } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
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
  const { t } = useI18n();

  return (
    <PageShell
      icon={Bot}
      title={t('agents.title')}
      subtitle={t('agents.subtitle')}
      width="lg"
      actions={
        <button
          type="button"
          className={s.addBtn}
          onClick={ctrl.beginCreate}
          disabled={ctrl.isEditing}
          data-testid="agents-add-btn"
        >
          <Icon icon={Plus} size={12} /> {t('agents.new')}
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
            title={t('agents.failed')}
            description={t('agents.failedDesc')}
          />
        </Card>
      ) : ctrl.defs.length === 0 && !ctrl.isEditing ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Bot} />}
            title={t('agents.empty')}
            description={t('agents.emptyDesc')}
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
  const { t } = useI18n();
  return (
    <div className={s.row} data-testid={`agent-row-${def.name}`}>
      <div className={s.rowMain}>
        <code className={s.rowName}>{def.name}</code>
        <span className={s.rowDesc}>{def.description}</span>
        {def.model && <Badge variant="accent">{def.model}</Badge>}
        {def.readonly && <Badge variant="neutral">{t('agents.readonly')}</Badge>}
        {def.spawnable && <Badge variant="success">{t('agents.spawnable')}</Badge>}
      </div>
      <div className={s.rowSide}>
        <div className={s.rowActions}>
          <button
            type="button"
            className={s.actionBtn}
            onClick={onEdit}
            title={t('agents.edit')}
            data-testid={`agent-edit-${def.name}`}
          >
            <Icon icon={Pencil} size={12} />
          </button>
          <button
            type="button"
            className={`${s.actionBtn} ${s.actionBtnDanger}`}
            onClick={onRemove}
            title={t('agents.delete')}
            data-testid={`agent-delete-${def.name}`}
          >
            <Icon icon={Trash2} size={12} />
          </button>
        </div>
      </div>
    </div>
  );
}

// 为测试方便再导出。
export { joinCsv, parseCsv };
