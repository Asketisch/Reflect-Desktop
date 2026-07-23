/**
 * Structured configuration form for ReflectConfig.
 *
 * Covers every section exposed by vendor/reflect-config/src/schema.rs
 * (25+ sections incl. mcp_servers / lsp_servers / hooks / routing /
 * subagent_providers / coordinator / postgres_session / sse_redis /
 * voice / dap / acp / feature_flags / ask_user_question / sanitize /
 * plugins / analytics / notifications / bridge / token_budget /
 * compact / sandbox / model / subagent providers).
 *
 * Pure utility: it reads/writes a TOML string via `parseFields` /
 * `applyField` so unknown keys remain untouched and the Advanced TOML
 * editor stays as a release valve. Backend validation still happens
 * via `reflect_save_config` -> `ReflectConfig::load_from_str`.
 */

import { Input, Select } from '@/features/design-system';

export type FieldKind = 'text' | 'number' | 'integer' | 'boolean' | 'select' | 'textarea';

export interface FieldSpec {
  /** Section key, e.g. 'anthropic' or 'compact'. */
  section: string;
  /** Field key inside the section. */
  key: string;
  label: string;
  kind: FieldKind;
  /** Options for `select`. */
  options?: string[];
  /** Whether the field represents a secret. */
  secret?: boolean;
  /** Optional placeholder. */
  placeholder?: string;
}

const SIMPLE_FIELDS: FieldSpec[] = [
  { section: 'active', key: 'provider', label: 'Active provider', kind: 'select', options: ['anthropic', 'openai', 'ollama'] },

  { section: 'anthropic', key: 'api_key', label: 'Anthropic API key', kind: 'text', secret: true, placeholder: 'sk-ant-...' },
  { section: 'anthropic', key: 'base_url', label: 'Anthropic base URL', kind: 'text', placeholder: 'https://api.anthropic.com' },
  { section: 'anthropic', key: 'model', label: 'Anthropic model', kind: 'text', placeholder: 'claude-3-5-sonnet-latest' },
  { section: 'anthropic', key: 'timeout_secs', label: 'Anthropic timeout (s)', kind: 'integer' },

  { section: 'openai', key: 'api_key', label: 'OpenAI API key', kind: 'text', secret: true, placeholder: 'sk-...' },
  { section: 'openai', key: 'base_url', label: 'OpenAI base URL', kind: 'text', placeholder: 'https://api.openai.com/v1' },
  { section: 'openai', key: 'model', label: 'OpenAI model', kind: 'text', placeholder: 'gpt-4o' },
  { section: 'openai', key: 'timeout_secs', label: 'OpenAI timeout (s)', kind: 'integer' },

  { section: 'ollama', key: 'base_url', label: 'Ollama base URL', kind: 'text', placeholder: 'http://127.0.0.1:11434' },
  { section: 'ollama', key: 'api_key', label: 'Ollama API key', kind: 'text', secret: true, placeholder: 'optional (Ollama Cloud)' },
  { section: 'ollama', key: 'model', label: 'Ollama model', kind: 'text', placeholder: 'llama3.2' },
  { section: 'ollama', key: 'keep_alive_secs', label: 'Ollama keep_alive_secs', kind: 'integer' },
  { section: 'ollama', key: 'num_ctx', label: 'Ollama num_ctx', kind: 'integer' },
  { section: 'ollama', key: 'num_gpu', label: 'Ollama num_gpu', kind: 'integer' },
  { section: 'ollama', key: 'timeout_secs', label: 'Ollama timeout (s)', kind: 'integer' },

  { section: 'compact', key: 'trigger_tokens', label: 'Compact trigger tokens', kind: 'integer' },

  { section: 'token_budget', key: 'session_total_tokens', label: 'Session total token budget', kind: 'integer' },
  { section: 'token_budget', key: 'per_turn_input_tokens', label: 'Per-turn input token limit', kind: 'integer' },

  { section: 'sandbox', key: 'os_level', label: 'Sandbox: OS-level (Seatbelt/Landlock)', kind: 'boolean' },
  { section: 'sandbox', key: 'allow_network', label: 'Sandbox: allow outbound network', kind: 'boolean' },

  { section: 'routing.main', key: 'primary', label: 'Routing: main primary', kind: 'text', placeholder: 'anthropic/claude-3-5-sonnet-latest' },
  { section: 'routing.compact', key: 'primary', label: 'Routing: compact primary', kind: 'text' },
  { section: 'routing.subagent', key: 'primary', label: 'Routing: subagent primary', kind: 'text' },

  { section: 'coordinator', key: 'enabled', label: 'Coordinator enabled', kind: 'select', options: ['inherit', 'true', 'false'] },
  { section: 'coordinator', key: 'system_prompt_path', label: 'Coordinator system prompt path', kind: 'text' },
  { section: 'coordinator', key: 'max_workers', label: 'Coordinator max workers', kind: 'integer' },

  { section: 'ask_user_question', key: 'enabled', label: 'AskUserQuestion enabled', kind: 'boolean' },
  { section: 'ask_user_question', key: 'max_questions', label: 'AskUserQuestion max questions', kind: 'integer' },
  { section: 'ask_user_question', key: 'max_options', label: 'AskUserQuestion max options', kind: 'integer' },
  { section: 'ask_user_question', key: 'default_timeout_secs', label: 'AskUserQuestion timeout (s)', kind: 'integer' },

  { section: 'model', key: 'context_window', label: 'Default model context window', kind: 'integer' },
  { section: 'model', key: 'input_price_micro_usd_per_mtok', label: 'Default input price (micro-USD/Mtok)', kind: 'integer' },
  { section: 'model', key: 'output_price_micro_usd_per_mtok', label: 'Default output price (micro-USD/Mtok)', kind: 'integer' },

  { section: 'analytics', key: 'enabled', label: 'Analytics enabled', kind: 'boolean' },
  { section: 'analytics', key: 'endpoint', label: 'Analytics OTLP endpoint', kind: 'text' },
  { section: 'analytics', key: 'service_name', label: 'Analytics service name', kind: 'text' },
  { section: 'analytics', key: 'flush_timeout_ms', label: 'Analytics flush timeout (ms)', kind: 'integer' },

  { section: 'notifications', key: 'webhook_url', label: 'Notifications webhook URL', kind: 'text' },

  { section: 'postgres_session', key: 'database_url', label: 'Postgres session database URL', kind: 'text', secret: true },
  { section: 'postgres_session', key: 'table_prefix', label: 'Postgres session table prefix', kind: 'text' },

  { section: 'sse_redis', key: 'redis_url', label: 'SSE Redis URL', kind: 'text' },
  { section: 'sse_redis', key: 'key_prefix', label: 'SSE Redis key prefix', kind: 'text' },

  { section: 'bridge', key: 'endpoint', label: 'Bridge endpoint', kind: 'text' },

  { section: 'voice', key: 'enabled', label: 'Voice enabled', kind: 'boolean' },
  { section: 'voice', key: 'provider', label: 'Voice provider', kind: 'text' },

  { section: 'dap', key: 'adapter', label: 'DAP adapter', kind: 'text' },

  { section: 'acp', key: 'bind', label: 'ACP bind address', kind: 'text', placeholder: '127.0.0.1:0' },

  { section: 'sanitize', key: 'enabled', label: 'Sanitize enabled', kind: 'boolean' },
  { section: 'sanitize', key: 'marker', label: 'Sanitize marker', kind: 'text' },
  { section: 'sanitize', key: 'disable_default_patterns', label: 'Sanitize disable default patterns', kind: 'boolean' },

  { section: 'plugins', key: 'enabled_plugins', label: 'Enabled plugins (one per line, e.g. code-formatter@anthropic-tools)', kind: 'textarea' },

  { section: 'config_version', key: 'version', label: 'Config version', kind: 'integer' },
];

export const SIMPLE_FIELDS_BY_SECTION: Record<string, FieldSpec[]> = SIMPLE_FIELDS.reduce(
  (acc, field) => {
    const list = acc[field.section] ?? [];
    list.push(field);
    acc[field.section] = list;
    return acc;
  },
  {} as Record<string, FieldSpec[]>,
);

const COMPLEX_SECTIONS = [
  'subagent_providers',
  'routing',
  'mcp_servers',
  'lsp_servers',
  'hooks',
  'feature_flags',
] as const;

export const STRUCTURED_SECTIONS: readonly string[] = Array.from(
  new Set([...COMPLEX_SECTIONS, ...Object.keys(SIMPLE_FIELDS_BY_SECTION)]),
);

/** Read a scalar/array field from a TOML string. */
export function readField(toml: string, section: string, key: string): string {
  const lines = toml.split('\n');
  let active: string | null = section === 'config_version' ? '' : null;
  for (const raw of lines) {
    const line = raw.trim();
    const sectionMatch = /^\[([^\]]+)\]/.exec(line);
    if (sectionMatch) {
      active = sectionMatch[1].trim() === section ? section : null;
      continue;
    }
    if (active === null) continue;
    const m = new RegExp(`^${escapeRegExp(key)}\\s*=\\s*(.+)$`).exec(line);
    if (!m) continue;
    const value = m[1].trim();
    if (section === 'plugins' && key === 'enabled_plugins' && value.startsWith('[') && value.endsWith(']')) {
      return value
        .slice(1, -1)
        .split(',')
        .map((v) => v.trim().replace(/^['"]|['"]$/g, ''))
        .filter(Boolean)
        .join('\n');
    }
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      return value.slice(1, -1);
    }
    return value;
  }
  if (section === 'config_version' && key === 'version') {
    const m = /^config_version\s*=\s*(\d+)/m.exec(toml);
    return m ? m[1] : '';
  }
  return '';
}

/** Apply a field change back into a TOML string. */
export function applyField(toml: string, section: string, key: string, value: string, kind: FieldKind): string {
  if (section === 'config_version' && key === 'version') {
    return replaceOrAppend(toml, /^config_version\s*=\s*\d+?$/m, `config_version = ${value || '1'}`);
  }
  if (kind === 'boolean') {
    const encoded = value === 'true' || value === 'on' ? 'true' : value === 'false' || value === 'off' ? 'false' : '';
    return setSectionKey(toml, section, key, encoded);
  }
  if (kind === 'integer' || kind === 'number') {
    const trimmed = value.trim();
    return setSectionKey(toml, section, key, trimmed === '' ? '' : trimmed);
  }
  if (kind === 'select') {
    if (section === 'coordinator' && key === 'enabled') {
      if (value === 'inherit') return removeSectionKey(toml, 'coordinator', 'enabled');
      return setSectionKey(toml, 'coordinator', 'enabled', value);
    }
    return setSectionKey(toml, section, key, value);
  }
  if (section === 'plugins' && key === 'enabled_plugins') {
    const items = value
      .split('\n')
      .map((v) => v.trim())
      .filter(Boolean);
    if (items.length === 0) return removeSectionKey(toml, 'plugins', 'enabled_plugins');
    const list = items.map((v) => `"${v.replace(/"/g, '\\"')}"`).join(', ');
    return setSectionKey(toml, 'plugins', 'enabled_plugins', list);
  }
  if (kind === 'textarea') {
    if (!value.trim()) return removeSectionKey(toml, section, key);
    return setSectionKey(toml, section, key, value.replace(/\n/g, '\\n'));
  }
  return setSectionKey(toml, section, key, value);
}

function setSectionKey(toml: string, section: string, key: string, encoded: string): string {
  if (!section.includes('.') && !encoded) return removeSectionKey(toml, section, key);
  const lines = toml.split('\n');
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (sectionStart !== -1 && sectionEnd === lines.length) sectionEnd = i;
      if (sec[1].trim() === section && sectionStart === -1) sectionStart = i;
    }
  }
  if (sectionStart === -1) {
    if (!encoded) return toml;
    return `${toml.replace(/\s*$/, '')}\n\n[${section}]\n${key} = ${encodeScalar(encoded)}\n`;
  }
  for (let i = sectionStart + 1; i < sectionEnd; i++) {
    if (new RegExp(`^${escapeRegExp(key)}\\s*=`).test(lines[i].trim())) {
      if (!encoded) {
        lines.splice(i, 1);
        return lines.join('\n');
      }
      lines[i] = `${key} = ${encodeScalar(encoded)}`;
      return lines.join('\n');
    }
  }
  if (!encoded) return lines.join('\n');
  lines.splice(sectionEnd, 0, `${key} = ${encodeScalar(encoded)}`);
  return lines.join('\n');
}

function removeSectionKey(toml: string, section: string, key: string): string {
  const lines = toml.split('\n');
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (sectionStart !== -1 && sectionEnd === lines.length) sectionEnd = i;
      if (sec[1].trim() === section && sectionStart === -1) sectionStart = i;
    }
  }
  if (sectionStart === -1) return toml;
  for (let i = sectionStart + 1; i < sectionEnd; i++) {
    if (new RegExp(`^${escapeRegExp(key)}\\s*=`).test(lines[i].trim())) {
      lines.splice(i, 1);
      return lines.join('\n');
    }
  }
  return lines.join('\n');
}

function encodeScalar(value: string): string {
  if (value === '') return '""';
  if (/^(true|false)$/.test(value)) return value;
  if (/^-?\d+(\.\d+)?$/.test(value)) return value;
  return JSON.stringify(value);
}

function replaceOrAppend(toml: string, pattern: RegExp, line: string): string {
  if (pattern.test(toml)) return toml.replace(pattern, line);
  return `${toml.replace(/\s*$/, '')}\n${line}\n`;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export interface StructuredFieldProps {
  spec: FieldSpec;
  toml: string;
  onChange: (value: string) => void;
  /** Controlled visibility override for secret fields. */
  showSecret?: boolean;
  id?: string;
}

/** Renders one structured input for a single field spec. */
export function StructuredField({ spec, toml, onChange, showSecret, id }: StructuredFieldProps) {
  const value = readField(toml, spec.section, spec.key);
  if (spec.kind === 'boolean') {
    const bool = value === 'true' || value === 'on';
    return (
      <input
        id={id}
        type="checkbox"
        checked={bool}
        aria-label={spec.label}
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
        aria-label={spec.label}
        value={value}
        placeholder={spec.placeholder}
        onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.value, spec.kind))}
      />
    );
  }
  return (
    <Input
      id={id}
      type={spec.secret ? (showSecret ? 'text' : 'password') : spec.kind === 'integer' || spec.kind === 'number' ? 'number' : 'text'}
      aria-label={spec.label}
      value={value}
      placeholder={spec.placeholder}
      onChange={(e) => onChange(applyField(toml, spec.section, spec.key, e.target.value, spec.kind))}
    />
  );
}

export { COMPLEX_SECTIONS };