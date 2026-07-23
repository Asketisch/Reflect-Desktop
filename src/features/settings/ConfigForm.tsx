/**
 * Comprehensive structured configuration form for ReflectConfig.
 *
 * Renders one input per FieldSpec across every vendor schema section
 * plus a categorized complex-section editor (subagent_providers /
 * routing / mcp_servers / lsp_servers / hooks / feature_flags /
 * plugins). Reads/writes the raw TOML via `applyField` so unknown
 * keys remain intact and the Advanced TOML editor stays the source
 * of truth for advanced fields.
 */

import { useMemo } from 'react';
import { Button, Input, Textarea } from '@/features/design-system';
import {
  COMPLEX_SECTIONS,
  SIMPLE_FIELDS_BY_SECTION,
  StructuredField,
  applyField,
  readField,
  type FieldSpec,
} from './configSchema';
import s from './SettingsView.module.css';

const COMPLEX_DESCRIPTIONS: Record<string, string> = {
  mcp_servers: '[mcp_servers.<name>] entries. Each entry is one section with command/url, args, env, headers, timeout_ms.',
  lsp_servers: '[lsp_servers.<name>] entries. Each entry is one section with command, file_patterns, root_uri, init options.',
  hooks: '[hooks] subsections: search_budget / test_runner / plan_completion / verification / langfuse_tracker / read_before_edit / enabled whitelist.',
  routing: '[routing.{main,compact,subagent}].primary + fallbacks + weights.',
  subagent_providers: '[subagent_providers.{anthropic,openai,ollama}] with independent api_key / base_url / model.',
  feature_flags: '[feature_flags] with `flags = { ... }` boolean table (edit via JSON).',
  plugins: '[plugins] enabled_plugins list + marketplace sources (edit via JSON below).',
};

const KNOWN_TOML_SECTIONS = new Set(Object.keys(SIMPLE_FIELDS_BY_SECTION).concat([...COMPLEX_SECTIONS]));

export function ConfigForm({ rawToml, onChange, showSecrets, onToggleSecret }: {
  rawToml: string;
  onChange: (value: string) => void;
  showSecrets: Record<string, boolean>;
  onToggleSecret: (key: string) => void;
}) {
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
                  <label className={s.fieldLabel} htmlFor={inputId}>{field.label}</label>
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
                        {(showSecrets[inputId] ?? false) ? 'Hide' : 'Show'}
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
      <UnknownSection rawToml={rawToml} onChange={onChange} />
    </div>
  );
}

function ComplexSection({ section, rawToml, onChange }: { section: string; rawToml: string; onChange: (v: string) => void }) {
  return (
    <section className={s.configSection} data-section={section}>
      <h4 className={s.configSectionTitle}>{section}</h4>
      <p className={s.sectionDesc}>{COMPLEX_DESCRIPTIONS[section]}</p>
      {section === 'feature_flags' ? (
        <FeatureFlagsEditor rawToml={rawToml} onChange={onChange} />
      ) : section === 'plugins' ? (
        <PluginsEditor rawToml={rawToml} onChange={onChange} />
      ) : (
        <ComplexSectionEditor section={section} rawToml={rawToml} onChange={onChange} />
      )}
    </section>
  );
}

function ComplexSectionEditor({ section, rawToml, onChange }: { section: string; rawToml: string; onChange: (v: string) => void }) {
  const entries = useMemo(() => extractSubSections(rawToml, section), [rawToml, section]);
  return (
    <div className={s.complexEditor}>
      {entries.length === 0 ? (
        <p className={s.hint}>No entries yet. Add one in the Advanced TOML editor.</p>
      ) : (
        entries.map((entry) => (
          <div key={entry.name} className={s.complexEntry}>
            <div className={s.complexEntryHeader}>
              <code className={s.codeInline}>{entry.name}</code>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => onChange(removeSubSection(rawToml, section, entry.name))}
              >
                Remove
              </Button>
            </div>
            <Textarea
              rows={4}
              spellCheck={false}
              value={entry.body}
              onChange={(e) => onChange(updateSubSection(rawToml, section, entry.name, e.target.value))}
            />
          </div>
        ))
      )}
      <Button
        size="sm"
        variant="ghost"
        onClick={() => onChange(addSubSection(rawToml, section, `${section}-new`))}
      >
        Add {section} entry
      </Button>
    </div>
  );
}

function FeatureFlagsEditor({ rawToml, onChange }: { rawToml: string; onChange: (v: string) => void }) {
  const flags = useMemo(() => extractFlags(rawToml), [rawToml]);
  return (
    <div className={s.complexEditor}>
      {flags.map((flag) => (
        <div key={flag.key} className={s.complexEntry}>
          <code className={s.codeInline}>{flag.key}</code>
          <input
            type="checkbox"
            checked={flag.value}
            aria-label={`Flag ${flag.key}`}
            onChange={(e) => onChange(setFlag(rawToml, flag.key, e.target.checked))}
          />
          <Button size="sm" variant="ghost" onClick={() => onChange(removeFlag(rawToml, flag.key))}>
            Remove
          </Button>
        </div>
      ))}
      <Input
        placeholder="new-flag-key"
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            const value = (e.target as HTMLInputElement).value.trim();
            if (value) onChange(addFlag(rawToml, value));
            (e.target as HTMLInputElement).value = '';
          }
        }}
      />
    </div>
  );
}

function PluginsEditor({ rawToml, onChange }: { rawToml: string; onChange: (v: string) => void }) {
  const enabled = readField(rawToml, 'plugins', 'enabled_plugins');
  return (
    <div className={s.complexEditor}>
      <label className={s.fieldLabel} htmlFor="plugins-enabled">enabled_plugins (one per line)</label>
      <Textarea
        id="plugins-enabled"
        rows={4}
        value={enabled}
        onChange={(e) => onChange(applyField(rawToml, 'plugins', 'enabled_plugins', e.target.value, 'textarea'))}
      />
    </div>
  );
}

function UnknownSection({ rawToml, onChange: _ }: { rawToml: string; onChange: (v: string) => void }) {
  const unknown = useMemo(() => findUnknownSections(rawToml), [rawToml]);
  if (unknown.length === 0) return null;
  return (
    <section className={s.configSection} data-section="unknown">
      <h4 className={s.configSectionTitle}>Other (advanced)</h4>
      <p className={s.sectionDesc}>
        These sections are not yet first-class inputs but remain editable. Use the Advanced TOML editor
        to modify them.
      </p>
      <ul className={s.unknownList}>
        {unknown.map((u) => (
          <li key={u}>{u}</li>
        ))}
      </ul>
    </section>
  );
}

// ====== Helpers ======

function extractSubSections(toml: string, prefix: string): { name: string; body: string }[] {
  const lines = toml.split('\n');
  const out: { name: string; body: string }[] = [];
  let current: { name: string; lines: string[] } | null = null;
  for (const raw of lines) {
    const line = raw.trim();
    const sec = /^\[([^\]]+)\]/.exec(line);
    if (sec) {
      if (current) out.push({ name: current.name, body: current.lines.join('\n').trim() });
      const full = sec[1].trim();
      if (full === prefix) current = { name: '(default)', lines: [] };
      else if (full.startsWith(`${prefix}.`)) current = { name: full.slice(prefix.length + 1), lines: [] };
      else current = null;
    } else if (current) {
      current.lines.push(raw);
    }
  }
  if (current) out.push({ name: current.name, body: current.lines.join('\n').trim() });
  return out.filter((e) => e.body.length > 0);
}

function updateSubSection(toml: string, prefix: string, name: string, body: string): string {
  if (name === '(default)') return setSectionBody(toml, `[${prefix}]`, body);
  return setSectionBody(toml, `[${prefix}.${name}]`, body);
}

function removeSubSection(toml: string, prefix: string, name: string): string {
  const target = name === '(default)' ? `[${prefix}]` : `[${prefix}.${name}]`;
  return removeSection(toml, target);
}

function addSubSection(toml: string, prefix: string, name: string): string {
  const header = name.includes('.') ? `[${prefix}.${name}]` : `[${prefix}]`;
  if (toml.includes(header)) return toml;
  return `${toml.replace(/\s*$/, '')}\n\n${header}\n# TODO: edit me\n`;
}

function setSectionBody(toml: string, header: string, body: string): string {
  const lines = toml.split('\n');
  const out: string[] = [];
  let replaced = false;
  let inTarget = false;
  let blockLines: string[] = [];
  for (const line of lines) {
    if (line.trim() === header) {
      if (inTarget) {
        out.push(...blockLines);
      }
      out.push(header);
      out.push(body.trimEnd());
      out.push('');
      inTarget = true;
      blockLines = [];
      replaced = true;
      continue;
    }
    if (inTarget) {
      if (/^\[/.test(line.trim())) {
        out.push(...blockLines);
        inTarget = false;
        blockLines = [];
      } else {
        blockLines.push(line);
        continue;
      }
    }
    out.push(line);
  }
  if (inTarget) out.push(...blockLines);
  if (!replaced) {
    out.push('');
    out.push(header);
    out.push(body.trimEnd());
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n');
}

function removeSection(toml: string, header: string): string {
  const lines = toml.split('\n');
  const out: string[] = [];
  let inTarget = false;
  for (const line of lines) {
    if (line.trim() === header) {
      inTarget = true;
      continue;
    }
    if (inTarget) {
      if (/^\[/.test(line.trim())) inTarget = false;
      else continue;
    }
    out.push(line);
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n');
}

function extractFlags(toml: string): { key: string; value: boolean }[] {
  const lines = toml.split('\n');
  let inSection = false;
  const out: { key: string; value: boolean }[] = [];
  for (const raw of lines) {
    const line = raw.trim();
    const sec = /^\[([^\]]+)\]/.exec(line);
    if (sec) {
      inSection = sec[1].trim() === 'feature_flags';
      continue;
    }
    if (!inSection) continue;
    const flagsMatch = /^flags\s*=\s*\{(.+)\}$/.exec(line);
    if (flagsMatch) {
      const pairs = flagsMatch[1].split(',');
      for (const pair of pairs) {
        const kv = /^"?([\w.-]+)"?\s*=\s*(true|false)/.exec(pair.trim());
        if (kv) out.push({ key: kv[1], value: kv[2] === 'true' });
      }
    } else {
      const inline = /^"?([\w.-]+)"?\s*=\s*(true|false)/.exec(line);
      if (inline) out.push({ key: inline[1], value: inline[2] === 'true' });
    }
  }
  return out;
}

function setFlag(toml: string, key: string, value: boolean): string {
  const lines = toml.split('\n');
  let start = -1;
  let end = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (start !== -1 && end === lines.length) end = i;
      if (sec[1].trim() === 'feature_flags' && start === -1) start = i;
    }
  }
  if (start === -1) {
    return `${toml.replace(/\s*$/, '')}\n\n[feature_flags]\nflags = { "${key}" = ${value} }\n`;
  }
  const body = lines.slice(start + 1, end);
  const flagsLine = body.find((l) => /^flags\s*=/.test(l.trim()));
  if (!flagsLine) {
    body.push(`flags = { "${key}" = ${value} }`);
    lines.splice(end, 0, ...body);
    return lines.join('\n');
  }
  const idx = body.indexOf(flagsLine);
  const inline = /\{(.+)\}/.exec(flagsLine);
  if (!inline) return toml;
  const entries = inline[1]
    .split(',')
    .map((p) => p.trim())
    .filter(Boolean);
  const filtered = entries.filter((p) => !new RegExp(`^"?${key}"?\\s*=`).test(p));
  filtered.push(`"${key}" = ${value}`);
  body[idx] = `flags = { ${filtered.join(', ')} }`;
  return [...lines.slice(0, start + 1), ...body, ...lines.slice(end)].join('\n');
}

function addFlag(toml: string, key: string): string {
  return setFlag(toml, key, true);
}

function removeFlag(toml: string, key: string): string {
  const lines = toml.split('\n');
  let start = -1;
  let end = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (start !== -1 && end === lines.length) end = i;
      if (sec[1].trim() === 'feature_flags' && start === -1) start = i;
    }
  }
  if (start === -1) return toml;
  const body = lines.slice(start + 1, end);
  const updated = body.filter((l) => !new RegExp(`"?${key}"?\\s*=\\s*(true|false)`).test(l.trim()));
  return [...lines.slice(0, start + 1), ...updated, ...lines.slice(end)].join('\n');
}

function findUnknownSections(toml: string): string[] {
  const sections = new Set<string>();
  for (const line of toml.split('\n')) {
    const sec = /^\[([^\]]+)\]/.exec(line.trim());
    if (!sec) continue;
    const key = sec[1].trim();
    const top = key.split('.')[0];
    if (!KNOWN_TOML_SECTIONS.has(key) && !KNOWN_TOML_SECTIONS.has(top)) sections.add(key);
  }
  return Array.from(sections);
}

export type { FieldSpec };