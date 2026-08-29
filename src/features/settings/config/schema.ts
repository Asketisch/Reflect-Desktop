/**
 * ReflectConfig 的纯结构化配置 schema。
 *
 * 覆盖 reflect-agent/crates/resources/reflect-config/src/schema.rs 暴露的每个分区
 * （25+ 个分区，包括 mcp_servers / lsp_servers / hooks / routing /
 * subagent_providers / coordinator / postgres_session / sse_redis /
 * voice / dap / acp / feature_flags / ask_user_question / sanitize /
 * plugins / analytics / notifications / bridge / token_budget /
 * compact / sandbox / model / subagent providers）。
 *
 * 纯工具：通过 `readField` / `applyField` 读写 TOML 字符串，
 * 未知键保持不变，高级 TOML 编辑器仍作为出口闸阀。后端校验仍
 * 通过 `reflect_save_config` -> `ReflectConfig::load_from_str` 完成。
 *
 * **i18n 约定**：`label` / `placeholder` 改为 `labelKey` / `placeholderKey`，渲染时
 * 通过 `t(labelKey)` 拿到当前 locale 文本。技术字段（skills / model / permission
 * mode id）保留原样。
 */

import type { LocaleKey } from '@/utils/i18n';

export type FieldKind = 'text' | 'number' | 'integer' | 'boolean' | 'select' | 'textarea';

export interface FieldSpec {
  /** 分区 key，如 'anthropic' 或 'compact'。 */
  section: string;
  /** 分区内的字段 key。 */
  key: string;
  /** 面向用户标签的 i18n key。 */
  labelKey: LocaleKey;
  kind: FieldKind;
  /** `select` 字段的选项。 */
  options?: string[];
  /** 该字段是否为密钥。 */
  secret?: boolean;
  /** 占位符的可选 i18n key。 */
  placeholderKey?: LocaleKey;
}

const SIMPLE_FIELDS: FieldSpec[] = [
  // provider = 接入端口（API 协议端点），仅 anthropic / openai 两种。
  // Ollama 走 OpenAI 兼容端口：base_url 指向本地服务即可，不再是独立选项。
  { section: 'active', key: 'provider', labelKey: 'settings.config.field.active.provider', kind: 'select', options: ['anthropic', 'openai'] },

  { section: 'anthropic', key: 'api_key', labelKey: 'settings.config.field.anthropic.api_key', kind: 'text', secret: true, placeholderKey: 'settings.config.placeholder.anthropic.api_key' },
  { section: 'anthropic', key: 'base_url', labelKey: 'settings.config.field.anthropic.base_url', kind: 'text', placeholderKey: 'settings.config.placeholder.anthropic.base_url' },
  { section: 'anthropic', key: 'model', labelKey: 'settings.config.field.anthropic.model', kind: 'text', placeholderKey: 'settings.config.placeholder.anthropic.model' },
  { section: 'anthropic', key: 'timeout_secs', labelKey: 'settings.config.field.anthropic.timeout_secs', kind: 'integer' },

  { section: 'openai', key: 'api_key', labelKey: 'settings.config.field.openai.api_key', kind: 'text', secret: true, placeholderKey: 'settings.config.placeholder.openai.api_key' },
  { section: 'openai', key: 'base_url', labelKey: 'settings.config.field.openai.base_url', kind: 'text', placeholderKey: 'settings.config.placeholder.openai.base_url' },
  { section: 'openai', key: 'model', labelKey: 'settings.config.field.openai.model', kind: 'text', placeholderKey: 'settings.config.placeholder.openai.model' },
  { section: 'openai', key: 'timeout_secs', labelKey: 'settings.config.field.openai.timeout_secs', kind: 'integer' },

  { section: 'compact', key: 'trigger_tokens', labelKey: 'settings.config.field.compact.trigger_tokens', kind: 'integer' },

  { section: 'token_budget', key: 'session_total_tokens', labelKey: 'settings.config.field.token_budget.session_total_tokens', kind: 'integer' },
  { section: 'token_budget', key: 'per_turn_input_tokens', labelKey: 'settings.config.field.token_budget.per_turn_input_tokens', kind: 'integer' },

  { section: 'sandbox', key: 'os_level', labelKey: 'settings.config.field.sandbox.os_level', kind: 'boolean' },
  { section: 'sandbox', key: 'allow_network', labelKey: 'settings.config.field.sandbox.allow_network', kind: 'boolean' },

  { section: 'routing.main', key: 'primary', labelKey: 'settings.config.field.routing.main.primary', kind: 'text', placeholderKey: 'settings.config.placeholder.routing.main.primary' },
  { section: 'routing.compact', key: 'primary', labelKey: 'settings.config.field.routing.compact.primary', kind: 'text' },
  { section: 'routing.subagent', key: 'primary', labelKey: 'settings.config.field.routing.subagent.primary', kind: 'text' },

  { section: 'coordinator', key: 'enabled', labelKey: 'settings.config.field.coordinator.enabled', kind: 'select', options: ['inherit', 'true', 'false'] },
  { section: 'coordinator', key: 'system_prompt_path', labelKey: 'settings.config.field.coordinator.system_prompt_path', kind: 'text' },
  { section: 'coordinator', key: 'max_workers', labelKey: 'settings.config.field.coordinator.max_workers', kind: 'integer' },

  { section: 'ask_user_question', key: 'enabled', labelKey: 'settings.config.field.ask_user_question.enabled', kind: 'boolean' },
  { section: 'ask_user_question', key: 'max_questions', labelKey: 'settings.config.field.ask_user_question.max_questions', kind: 'integer' },
  { section: 'ask_user_question', key: 'max_options', labelKey: 'settings.config.field.ask_user_question.max_options', kind: 'integer' },
  { section: 'ask_user_question', key: 'default_timeout_secs', labelKey: 'settings.config.field.ask_user_question.default_timeout_secs', kind: 'integer' },

  { section: 'model', key: 'context_window', labelKey: 'settings.config.field.model.context_window', kind: 'integer' },

  { section: 'analytics', key: 'enabled', labelKey: 'settings.config.field.analytics.enabled', kind: 'boolean' },
  { section: 'analytics', key: 'endpoint', labelKey: 'settings.config.field.analytics.endpoint', kind: 'text' },
  { section: 'analytics', key: 'service_name', labelKey: 'settings.config.field.analytics.service_name', kind: 'text' },
  { section: 'analytics', key: 'flush_timeout_ms', labelKey: 'settings.config.field.analytics.flush_timeout_ms', kind: 'integer' },

  { section: 'notifications', key: 'webhook_url', labelKey: 'settings.config.field.notifications.webhook_url', kind: 'text' },

  { section: 'postgres_session', key: 'database_url', labelKey: 'settings.config.field.postgres_session.database_url', kind: 'text', secret: true },
  { section: 'postgres_session', key: 'table_prefix', labelKey: 'settings.config.field.postgres_session.table_prefix', kind: 'text' },

  { section: 'sse_redis', key: 'redis_url', labelKey: 'settings.config.field.sse_redis.redis_url', kind: 'text' },
  { section: 'sse_redis', key: 'key_prefix', labelKey: 'settings.config.field.sse_redis.key_prefix', kind: 'text' },

  { section: 'bridge', key: 'endpoint', labelKey: 'settings.config.field.bridge.endpoint', kind: 'text' },

  { section: 'voice', key: 'enabled', labelKey: 'settings.config.field.voice.enabled', kind: 'boolean' },
  { section: 'voice', key: 'provider', labelKey: 'settings.config.field.voice.provider', kind: 'text' },

  { section: 'dap', key: 'adapter', labelKey: 'settings.config.field.dap.adapter', kind: 'text' },

  { section: 'acp', key: 'bind', labelKey: 'settings.config.field.acp.bind', kind: 'text', placeholderKey: 'settings.config.placeholder.acp.bind' },

  { section: 'sanitize', key: 'enabled', labelKey: 'settings.config.field.sanitize.enabled', kind: 'boolean' },
  { section: 'sanitize', key: 'marker', labelKey: 'settings.config.field.sanitize.marker', kind: 'text' },
  { section: 'sanitize', key: 'disable_default_patterns', labelKey: 'settings.config.field.sanitize.disable_default_patterns', kind: 'boolean' },

  { section: 'plugins', key: 'enabled_plugins', labelKey: 'settings.config.field.plugins.enabled_plugins', kind: 'textarea' },

  { section: 'config_version', key: 'version', labelKey: 'settings.config.field.config_version.version', kind: 'integer' },
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

export const COMPLEX_SECTIONS = [
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

/** 从 TOML 字符串读取标量 / 数组字段。 */
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

/** 将字段变更写回 TOML 字符串。 */
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