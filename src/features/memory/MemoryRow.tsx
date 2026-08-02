/**
 * Memory —— MemoryRow (presentational, B11-01 refactor).
 *
 * Extracted from MemoryView; behavior preserved. The danger color for the
 * delete button now uses the existing `--danger` token via the
 * `.actionBtnDanger` CSS class instead of an inline hex.
 */
import { Plus, Trash2, Save, X } from 'lucide-react';
import { Card, Badge, Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import type { ReflectMemoryEntry } from '@/utils/commands';
import { entryKey } from './useMemoryController';
import s from './MemoryView.module.css';

export interface MemoryRowProps {
  entry: ReflectMemoryEntry;
  isEditing: boolean;
  editValue: string;
  /** Flip into edit mode for this entry (controller initializes the buffer). */
  onEdit: () => void;
  /** Update the in-progress edit buffer. */
  onChangeEdit: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
  onRemove: () => void;
}

export function MemoryRow({
  entry,
  isEditing,
  editValue,
  onEdit,
  onChangeEdit,
  onSave,
  onCancel,
  onRemove,
}: MemoryRowProps) {
  const { t } = useI18n();

  return (
    <Card level="outlined" padding="sm" className={s.row} data-testid={`memory-entry-${entry.key}`}>
      <div className={s.rowLeft}>
        <Badge variant="neutral">{entry.scope}</Badge>
        <code className={s.key}>{entry.key}</code>
      </div>
      {isEditing ? (
        <div className={s.editRow} data-testid={`memory-edit-${entryKey(entry)}`}>
          <textarea
            value={editValue}
            onChange={(e) => onChangeEdit(e.target.value)}
            className={s.editInput}
            rows={2}
            autoFocus
            data-testid="memory-edit-input"
          />
          <div className={s.editActions}>
            <button type="button" className={s.saveBtnSmall} onClick={onSave} data-testid="memory-edit-save">
              <Icon icon={Save} size={11} /> {t('memory.save')}
            </button>
            <button type="button" className={s.cancelBtnSmall} onClick={onCancel}>
              <Icon icon={X} size={11} />
            </button>
          </div>
        </div>
      ) : (
        <div className={s.rowRight}>
          <span className={s.value} title={entry.value}>
            {entry.value}
          </span>
          <div className={s.actions}>
            <button
              type="button"
              className={s.actionBtn}
              onClick={onEdit}
              title={t('memory.edit')}
            >
              <Icon icon={Plus} size={11} />
            </button>
            <button
              type="button"
              className={`${s.actionBtn} ${s.actionBtnDanger}`}
              onClick={onRemove}
              title={t('memory.delete')}
              data-testid="memory-remove"
            >
              <Icon icon={Trash2} size={11} />
            </button>
          </div>
        </div>
      )}
    </Card>
  );
}