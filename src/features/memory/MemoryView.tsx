/**
 * Memory —— 持久记忆管理面板 (B11-01).
 *
 * 通过 `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory`
 * 与后端同步。支持 scope (global / project / session) 过滤,
 * 内联编辑 + 删除 + 新增表单。
 *
 * 重构于 2026-07-25：查询/变更编排位于 `useMemoryController`；
 * 行和表单渲染位于 `MemoryRow` / `MemoryAddForm`。
 * 本组件用过滤栏和空态/错误/加载状态将它们组合起来。
 */
import { Brain, Plus, X } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Icon, EmptyState, Spinner } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import {
  FILTERS,
  SCOPE_KEYS,
  entryKey,
  useMemoryController,
} from './useMemoryController';
import { MemoryRow } from './MemoryRow';
import { MemoryAddForm } from './MemoryAddForm';
import s from './MemoryView.module.css';

export function MemoryView() {
  const { t } = useI18n();
  const ctrl = useMemoryController({ t });

  return (
    <PageShell
      icon={Brain}
      title={t('memory.title')}
      subtitle={t('memory.subtitle')}
      width="lg"
    >
      <div className={s.filterBar}>
        {FILTERS.map((sc) => (
          <button
            key={sc}
            type="button"
            className={s.filterBtn}
            data-active={ctrl.filter === sc || undefined}
            onClick={() => ctrl.setFilter(sc)}
            data-testid={`memory-filter-${sc}`}
          >
            {t(SCOPE_KEYS[sc])} <span className={s.count}>{ctrl.counts[sc] ?? 0}</span>
          </button>
        ))}
        <button
          type="button"
          className={s.addBtn}
          onClick={ctrl.toggleShowForm}
          data-testid="memory-add-btn"
        >
          {ctrl.showForm ? <Icon icon={X} size={12} /> : <Icon icon={Plus} size={12} />}{' '}
          {ctrl.showForm ? t('memory.cancel') : t('memory.add')}
        </button>
      </div>

      {ctrl.showForm && (
        <MemoryAddForm
          newScope={ctrl.newScope}
          newKey={ctrl.newKey}
          newValue={ctrl.newValue}
          setNewScope={ctrl.setNewScope}
          setNewKey={ctrl.setNewKey}
          setNewValue={ctrl.setNewValue}
          onSubmit={() => void ctrl.addNew()}
        />
      )}

      {ctrl.loading ? (
        <div className={s.loading}>
          <Spinner size={20} />
        </div>
      ) : ctrl.error ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Brain} />}
            title={t('memory.failed')}
            description={t('memory.failedDesc')}
          />
        </Card>
      ) : ctrl.filtered.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={Brain} />}
            title={t('memory.empty')}
            description={t('memory.emptyDesc')}
          />
        </Card>
      ) : (
        <div className={s.list} data-testid="memory-list">
          {ctrl.filtered.map((entry) => {
            const key = entryKey(entry);
            return (
              <MemoryRow
                key={key}
                entry={entry}
                isEditing={ctrl.editingKey === key}
                editValue={ctrl.editValue}
                onEdit={() => ctrl.beginEdit(entry)}
                onChangeEdit={ctrl.setEditValue}
                onSave={() => void ctrl.saveEdit(entry)}
                onCancel={ctrl.cancelEdit}
                onRemove={() => void ctrl.remove(entry)}
              />
            );
          })}
        </div>
      )}
    </PageShell>
  );
}
