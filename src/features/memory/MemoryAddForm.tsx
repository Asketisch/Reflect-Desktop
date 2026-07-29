/**
 * Memory —— AddForm (presentational, B11-01 refactor).
 *
 * Stateless form for creating a new memory entry. Behavior preserved from
 * the inline form previously rendered inside MemoryView.
 */
import { Save } from 'lucide-react';
import { Card, Icon } from '@/features/design-system';
import { NEW_SCOPES } from './useMemoryController';
import s from './MemoryView.module.css';

export interface MemoryAddFormProps {
  newScope: 'user' | 'project';
  newKey: string;
  newValue: string;
  setNewScope: (s: 'user' | 'project') => void;
  setNewKey: (v: string) => void;
  setNewValue: (v: string) => void;
  onSubmit: () => void;
}

export function MemoryAddForm({
  newScope,
  newKey,
  newValue,
  setNewScope,
  setNewKey,
  setNewValue,
  onSubmit,
}: MemoryAddFormProps) {
  return (
    <Card level="outlined" padding="sm" className={s.addForm}>
      <div className={s.formRow}>
        <select
          value={newScope}
          onChange={(e) => setNewScope(e.target.value as 'user' | 'project')}
          className={s.formSelect}
          data-testid="memory-new-scope"
        >
          {NEW_SCOPES.map((sc) => (
            <option key={sc} value={sc}>
              {sc.charAt(0).toUpperCase() + sc.slice(1)}
            </option>
          ))}
        </select>
        <input
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          className={s.formInput}
          placeholder="Key (e.g. preference)"
          data-testid="memory-new-key"
        />
        <input
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          className={s.formInput}
          placeholder="Value"
          data-testid="memory-new-value"
        />
        <button type="button" className={s.saveBtn} onClick={onSubmit} data-testid="memory-add-confirm">
          <Icon icon={Save} size={12} /> Save
        </button>
      </div>
    </Card>
  );
}
