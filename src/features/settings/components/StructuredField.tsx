/**
 * Renders one structured input for a single `FieldSpec`.
 *
 * Lives in `components/` (it is React/DOM-bound) so the pure TOML
 * helpers in `config/` stay unit-testable without React.
 */

import { Input, Select } from '@/features/design-system';
import { t as i18nT } from '@/utils/i18n';
import { applyField, readField, type FieldSpec } from '../config/schema';

export interface StructuredFieldProps {
  spec: FieldSpec;
  toml: string;
  onChange: (value: string) => void;
  /** Controlled visibility override for secret fields. */
  showSecret?: boolean;
  id?: string;
}

export function StructuredField({ spec, toml, onChange, showSecret, id }: StructuredFieldProps) {
  const value = readField(toml, spec.section, spec.key);
  const label = i18nT(spec.labelKey);
  const placeholder = spec.placeholderKey ? i18nT(spec.placeholderKey) : undefined;
  if (spec.kind === 'boolean') {
    const bool = value === 'true' || value === 'on';
    return (
      <input
        id={id}
        type="checkbox"
        checked={bool}
        aria-label={label}
        onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.checked ? 'true' : 'false', spec.kind))}
      />
    );
  }
  if (spec.kind === 'select' && spec.options) {
    const raw = value || (spec.options[0] ?? '');
    return (
      <Select id={id} value={raw} onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.value, spec.kind))}>
        {spec.options.map((opt) => (
          <option key={opt} value={opt}>{opt}</option>
        ))}
      </Select>
    );
  }
  if (spec.kind === 'textarea') {
    return (
      <textarea
        id={id}
        rows={3}
        aria-label={label}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.value, spec.kind))}
      />
    );
  }
  return (
    <Input
      id={id}
      type={spec.secret ? (showSecret ? 'text' : 'password') : spec.kind === 'integer' || spec.kind === 'number' ? 'number' : 'text'}
      aria-label={label}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.value, spec.kind))}
    />
  );
}