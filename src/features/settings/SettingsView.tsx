/**
 * Settings —— 阶段 4:真实持久化到 ~/.reflect/config.toml。
 *
 * 通过 `reflect_get_config` / `reflect_save_config` 读写后端 config。
 * - 结构化快捷编辑:active provider / anthropic+openai api_key / model / permission mode
 * - 高级:原始 TOML 编辑器(MCP / LSP / sanitize / routing 等段)
 *
 * 保存时:把表单字段 merge 回原始 TOML(逐行替换已知字段),保留其它段不变,
 * 调 `reflect_save_config(toml)`。后端写盘前用 load_from_str 校验合法性。
 */
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_get_config,
  reflect_save_config,
  reflect_agent_status,
  reflect_set_permission_mode,
} from '@/utils/tauri';

type PermissionMode = 'auto' | 'prompt' | 'deny' | 'plan';

export function SettingsView({ onClose }: { onClose?: () => void }) {
  const qc = useQueryClient();
  const [rawToml, setRawToml] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 加载真实 config。
  const configQuery = useQuery({
    queryKey: ['config'],
    queryFn: reflect_get_config,
    staleTime: 0,
  });
  const statusQuery = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  // config 加载后初始化 rawToml + 解析字段。
  useEffect(() => {
    if (configQuery.data) setRawToml(configQuery.data);
  }, [configQuery.data]);

  // 结构化字段(从 rawToml 解析)。
  const fields = useMemo(() => parseFields(rawToml), [rawToml]);

  const saveMutation = useMutation({
    mutationFn: async (toml: string) => reflect_save_config(toml),
    onSuccess: () => {
      setSaved(true);
      setError(null);
      setTimeout(() => setSaved(false), 2500);
      qc.invalidateQueries({ queryKey: ['config'] });
      qc.invalidateQueries({ queryKey: ['agent-status'] });
    },
    onError: (e: unknown) => {
      setError(e instanceof Error ? e.message : String(e));
    },
  });

  const permMutation = useMutation({
    mutationFn: async (mode: string) => reflect_set_permission_mode(mode),
    onSuccess: () => {
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    },
  });

  /** 把结构化字段写回 rawToml(逐行替换),返回新 TOML。 */
  const buildToml = (next: Partial<Fields>): string => mergeFields(rawToml, { ...fields, ...next });

  const onSaveProvider = () => {
    saveMutation.mutate(buildToml({}));
  };
  const onPermission = (mode: PermissionMode) => {
    permMutation.mutate(mode);
  };

  const status = statusQuery.data;

  return (
    <div style={{ padding: 32, maxWidth: 680, margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16 }}>
        <h1 style={{ fontSize: 22, margin: 0, flex: 1 }}>Settings</h1>
        {onClose && (
          <button onClick={onClose} style={{ padding: '4px 12px', cursor: 'pointer' }}>
            Close
          </button>
        )}
      </div>

      {/* agent 状态徽标 */}
      <div
        style={{
          padding: 10,
          marginBottom: 16,
          borderRadius: 6,
          background: status?.has_model ? '#dcfce7' : '#fef3c7',
          border: `1px solid ${status?.has_model ? '#22c55e' : '#f59e0b'}`,
          fontSize: 12,
        }}
      >
        {configQuery.isLoading || statusQuery.isLoading ? (
          'Loading status…'
        ) : status?.has_model ? (
          <>✓ Agent ready — model <code>{status.model}</code></>
        ) : (
          <>
            ⚠ Degraded — {status?.degraded_reason ?? 'no provider configured'}.
            Set an API key below.
          </>
        )}
      </div>

      {error && (
        <div
          style={{
            padding: 10,
            marginBottom: 16,
            borderRadius: 6,
            background: '#fee2e2',
            border: '1px solid #ef4444',
            color: '#991b1b',
            fontSize: 12,
          }}
        >
          ⚠ {error}
        </div>
      )}

      <Section title="Provider">
        <label style={{ display: 'block', marginBottom: 12 }}>
          <span style={{ fontSize: 12, color: '#666' }}>Active provider</span>
          <select
            value={fields.activeProvider}
            onChange={(e) => {
              const next = buildToml({ activeProvider: e.target.value });
              setRawToml(next);
            }}
            style={{ display: 'block', marginTop: 4, padding: 6, fontSize: 13, width: '100%' }}
          >
            <option value="anthropic">Anthropic</option>
            <option value="openai">OpenAI</option>
            <option value="ollama">Ollama (local)</option>
          </select>
        </label>

        <Field
          label="Anthropic API key"
          value={fields.anthropicKey}
          placeholder="sk-ant-..."
          onChange={(v) => setRawToml(buildToml({ anthropicKey: v }))}
        />
        <Field
          label="Anthropic model"
          value={fields.anthropicModel}
          placeholder="claude-3-5-sonnet-latest"
          onChange={(v) => setRawToml(buildToml({ anthropicModel: v }))}
        />

        <Field
          label="OpenAI API key"
          value={fields.openaiKey}
          placeholder="sk-..."
          onChange={(v) => setRawToml(buildToml({ openaiKey: v }))}
        />
        <Field
          label="OpenAI model"
          value={fields.openaiModel}
          placeholder="gpt-4o"
          onChange={(v) => setRawToml(buildToml({ openaiModel: v }))}
        />

        <p style={{ fontSize: 11, color: '#888', marginTop: 8 }}>
          也可设环境变量 <code>ANTHROPIC_API_KEY</code> / <code>OPENAI_API_KEY</code> /
          <code>OLLAMA_HOST</code>,无需写进配置文件。
        </p>
      </Section>

      <Section title="Permissions">
        <div style={{ display: 'flex', gap: 8, marginBottom: 8, flexWrap: 'wrap' }}>
          {(['auto', 'prompt', 'deny', 'plan'] as PermissionMode[]).map((m) => (
            <button
              key={m}
              onClick={() => onPermission(m)}
              style={{
                padding: '6px 14px',
                border: '1px solid #e2e8f0',
                borderRadius: 6,
                cursor: 'pointer',
                fontSize: 13,
                background: 'white',
              }}
            >
              {m}
            </button>
          ))}
        </div>
        <p style={{ fontSize: 11, color: '#888' }}>
          auto: 直接运行 · prompt: 需审批 · deny: 禁止 · plan: 只读规划
        </p>
      </Section>

      <Section title="Advanced (raw TOML)">
        <button
          onClick={() => setShowAdvanced((s) => !s)}
          style={{ padding: '4px 10px', fontSize: 12, cursor: 'pointer', marginBottom: 8 }}
        >
          {showAdvanced ? 'Hide' : 'Show'} raw config (MCP / LSP / sanitize / routing)
        </button>
        {showAdvanced && (
          <>
            <textarea
              value={rawToml}
              onChange={(e) => setRawToml(e.target.value)}
              style={{
                width: '100%',
                minHeight: 280,
                padding: 10,
                fontFamily: 'ui-monospace, monospace',
                fontSize: 12,
                borderRadius: 6,
                border: '1px solid #cbd5e1',
              }}
              spellCheck={false}
            />
            <p style={{ fontSize: 11, color: '#888' }}>
              编辑 [mcp_servers.&lt;name&gt;] / [lsp_servers.&lt;name&gt;] 等段。保存后需
              重启 app 拉起新的 MCP/LSP server(热重载在后续阶段接入)。
            </p>
          </>
        )}
      </Section>

      <button
        onClick={onSaveProvider}
        disabled={saveMutation.isPending || configQuery.isLoading}
        style={{
          padding: '10px 20px',
          background: saveMutation.isPending ? '#93c5fd' : '#3b82f6',
          color: 'white',
          border: 'none',
          borderRadius: 6,
          cursor: saveMutation.isPending ? 'wait' : 'pointer',
          fontSize: 14,
        }}
      >
        {saveMutation.isPending
          ? 'Saving…'
          : saved
            ? 'Saved ✓'
            : 'Save to ~/.reflect/config.toml'}
      </button>
    </div>
  );
}

// ====== TOML 字段解析 / 合并(轻量正则,避免引入 toml 库) ======

interface Fields {
  activeProvider: string;
  anthropicKey: string;
  anthropicModel: string;
  openaiKey: string;
  openaiModel: string;
}

function parseFields(toml: string): Fields {
  return {
    activeProvider: readField(toml, 'provider', 'anthropic') || 'anthropic',
    anthropicKey: readField(toml, 'api_key', '', 'anthropic'),
    anthropicModel: readField(toml, 'model', '', 'anthropic'),
    openaiKey: readField(toml, 'api_key', '', 'openai'),
    openaiModel: readField(toml, 'model', '', 'openai'),
  };
}

/**
 * 读取 TOML 字段。section 不给时读顶层(如 [active] provider);
 * section 给定时读该 section 内(如 [anthropic] api_key)。
 */
function readField(toml: string, key: string, fallback: string, section?: string): string {
  const lines = toml.split('\n');
  let inSection = !section; // 无 section → 只读顶层(遇到 [x] 之前)。
  for (const line of lines) {
    const trimmed = line.trim();
    const sectionMatch = /^\[([^\]]+)\]/.exec(trimmed);
    if (sectionMatch) {
      inSection = section ? sectionMatch[1].trim() === section : false;
      continue;
    }
    if (!inSection) continue;
    const m = new RegExp(`^${key}\\s*=\\s*(.+)$`).exec(trimmed);
    if (m) {
      let v = m[1].trim();
      // 去引号。
      if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) {
        v = v.slice(1, -1);
      }
      return v;
    }
  }
  return fallback;
}

/**
 * 把字段写回 TOML。对每个字段:找到对应 section + key 行替换;找不到则追加 section。
 * 这是最小破坏性合并 —— 保留其它所有段和注释。
 */
function mergeFields(original: string, f: Fields): string {
  let out = original;
  out = ensureActiveProvider(out, f.activeProvider);
  out = setInSection(out, 'anthropic', 'api_key', f.anthropicKey);
  out = setInSection(out, 'anthropic', 'model', f.anthropicModel);
  out = setInSection(out, 'openai', 'api_key', f.openaiKey);
  out = setInSection(out, 'openai', 'model', f.openaiModel);
  return out;
}

/** 顶层 [active] provider = "..." */
function ensureActiveProvider(toml: string, provider: string): string {
  const lines = toml.split('\n');
  let inActive = false;
  let setAt = -1;
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    const sec = /^\[([^\]]+)\]/.exec(trimmed);
    if (sec) {
      inActive = sec[1].trim() === 'active';
      continue;
    }
    if (inActive && /^provider\s*=/.test(trimmed)) {
      lines[i] = `provider = "${provider}"`;
      setAt = i;
      break;
    }
  }
  if (setAt === -1) {
    // 没有 [active] 段或 provider 字段 —— 追加。
    lines.push('');
    lines.push('[active]');
    lines.push(`provider = "${provider}"`);
  }
  return lines.join('\n');
}

/** 在指定 section 内设 key = "value";section 不存在则追加。 */
function setInSection(toml: string, section: string, key: string, value: string): string {
  const lines = toml.split('\n');
  // 空值时跳过(不写空 key)。
  if (value === '') {
    // 但若原有该 key,保留原值(不删)。
    return lines.join('\n');
  }
  let sectionStart = -1;
  let sectionEnd = lines.length;
  let found = false;
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    const sec = /^\[([^\]]+)\]/.exec(trimmed);
    if (sec) {
      if (sectionStart !== -1 && sectionEnd === lines.length) {
        sectionEnd = i;
      }
      if (sec[1].trim() === section && sectionStart === -1) {
        sectionStart = i;
      }
    }
  }
  if (sectionStart === -1) {
    // section 不存在 → 追加。
    lines.push('');
    lines.push(`[${section}]`);
    lines.push(`${key} = "${value}"`);
    return lines.join('\n');
  }
  // section 存在 → 在 [sectionStart, sectionEnd) 内找 key。
  for (let i = sectionStart + 1; i < sectionEnd; i++) {
    if (new RegExp(`^${key}\\s*=`).test(lines[i].trim())) {
      lines[i] = `${key} = "${value}"`;
      found = true;
      break;
    }
  }
  if (!found) {
    lines.splice(sectionEnd, 0, `${key} = "${value}"`);
  }
  return lines.join('\n');
}

// ====== UI 子组件 ======

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section style={{ marginBottom: 24 }}>
      <h2 style={{ fontSize: 14, marginBottom: 8, color: '#666' }}>{title}</h2>
      {children}
    </section>
  );
}

function Field({
  label,
  value,
  placeholder,
  onChange,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
}) {
  return (
    <label style={{ display: 'block', marginBottom: 10 }}>
      <span style={{ fontSize: 12, color: '#666' }}>{label}</span>
      <input
        type="password"
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        style={{
          display: 'block',
          marginTop: 4,
          padding: 6,
          fontSize: 13,
          width: '100%',
          fontFamily: 'ui-monospace, monospace',
          border: '1px solid #cbd5e1',
          borderRadius: 4,
        }}
      />
    </label>
  );
}
