/**
 * Comprehensive structured configuration form for ReflectConfig.
 *
 * Renders one input per FieldSpec across every vendor schema section
 * plus a categorized complex-section editor (subagent_providers /
 * routing / mcp_servers / lsp_servers / hooks / feature_flags /
 * plugins). Reads/writes the raw TOML via pure helpers in `./config`
 * so unknown keys remain intact and the Advanced TOML editor stays
 * the source of truth for advanced fields.
 */

import { useMemo } from 'react';
import { Button } from '@/features/design-system';
import { useI18n, type LocaleKey } from '@/utils/i18n';
import {
  COMPLEX_SECTIONS,
  SIMPLE_FIELDS_BY_SECTION,
  type FieldSpec,
} from './config/schema';
import { StructuredField } from './components/StructuredField';
import { ComplexSection, UnknownSection } from './components/ComplexEditors';
import s from './SettingsView.module.css';

const KNOWN_TOML_SECTIONS = new Set(
  Object.keys(SIMPLE_FIELDS_BY_SECTION).concat([...COMPLEX_SECTIONS]),
);

export function ConfigForm({ rawToml, onChange, showSecrets, onToggleSecret }: {
  rawToml: string;
  onChange: (value: string) => void;
  showSecrets: Record<string, boolean>;
  onToggleSecret: (key: string) => void;
}) {
  const { t } = useI18n();
  const sections = useMemo(() => Object.keys(SIMPLE_FIELDS_BY_SECTION), []);
  return (
    <div className={s.configForm}>
      {sections.map((section) => (
        <section key={section} className={s.configSection} data-section={section}>
          <h4 className={s.configSectionTitle}>{section}</h4>
          <div className={s.providerFields}>
            {SIMPLE_FIELDS_BY_SECTION[section].map((field) => {
              const inputId = `config-${field.section}-${field.key}`;
              return (
                <div key={inputId}>
                  <label className={s.fieldLabel} htmlFor={inputId}>{t(field.labelKey as LocaleKey)}</label>
                  <div className={s.configInputRow}>
                    <StructuredField
                      id={inputId}
                      spec={field}
                      toml={rawToml}
                      onChange={onChange}
                      showSecret={showSecrets[inputId] ?? false}
                    />
                    {field.secret ? (
                      <Button size="sm" variant="ghost" onClick={() => onToggleSecret(inputId)}>
                        {(showSecrets[inputId] ?? false) ? t('settings.config.hide') : t('settings.config.show')}
                      </Button>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      ))}
      {COMPLEX_SECTIONS.map((section) => (
        <ComplexSection key={section} section={section} rawToml={rawToml} onChange={onChange} />
      ))}
      <UnknownSection rawToml={rawToml} knownTopLevel={KNOWN_TOML_SECTIONS} onChange={onChange} />
    </div>
  );
}

export type { FieldSpec };