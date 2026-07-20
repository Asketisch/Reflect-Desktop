/**
 * Settings —— IDE 式二级导航 + 卡片内容（CSS Modules 版）。
 *
 * 左侧二级 nav:Provider / Permissions / Advanced。
 * 右侧内容区:对应 section 的卡片化表单。
 *
 * 契约（SettingsView.test.tsx）：
 *   - 'Settings' / 'Provider' / 'Permissions' / 'Advanced' 文字
 *   - 'Agent ready' 状态徽标
 *   - permission 按钮 'plan' 触发 reflect_set_permission_mode
 *   - 'Save to ~/.reflect/config.toml' 按钮触发 reflect_save_config
 */
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, AlertTriangle, Eye, EyeOff } from 'lucide-react';
import {
  reflect_get_config,
  reflect_save_config,
  reflect_agent_status,
  reflect_set_permission_mode,
} from '@/utils/tauri';
import { Button, Input, Textarea, Badge, Icon, IconButton } from '@/features/design-system';
import s from './SettingsView.module.css';

type PermissionMode = 'auto' | 'prompt' | 'deny' | 'plan';
type Section = 'provider' | 'permissions' | 'advanced';

export function SettingsView({ onClose }: { onClose?: () => void }) {
  const qc = useQueryClient();
  const [section, setSection] = useState<Section>('provider');
  const [rawToml, setRawToml] = useState('');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});

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

  useEffect(() => {
    if (configQuery.data) setRawToml(configQuery.data);
  }, [configQuery.data]);

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

  const buildToml = (next: Partial<Fields>): string => mergeFields(rawToml, { ...fields, ...next });
  const onSave = () => saveMutation.mutate(buildToml({}));
  const onPermission = (mode: PermissionMode) => permMutation.mutate(mode);

  const status = statusQuery.data;
  const hasModel = Boolean(status?.has_model);

  return (
    <div className={s.root}>
      <aside className={s.nav}>
        <h2 className={s.title}>Settings</h2>
        <nav className={s.navList}>
          <NavBtn active={section === 'provider'} onClick={() => setSection('provider')}>
            Provider
          </NavBtn>
          <NavBtn active={section === 'permissions'} onClick={() => setSection('permissions')}>
            Permissions
          </NavBtn>
          <NavBtn active={section === 'advanced'} onClick={() => setSection('advanced')}>
            Advanced
          </NavBtn>
        </nav>
        {onClose && (
          <Button variant="ghost" size="sm" onClick={onClose} className={s.closeBtn}>
            Close
          </Button>
        )}
      </aside>

      <main className={s.content}>
        {/* 状态徽标 —— 契约 'Agent ready' */}
        <div className={s.statusBar} data-ok={hasModel || undefined}>
          <Icon icon={hasModel ? CheckCircle2 : AlertTriangle} size={14} />
          <span>
            {configQuery.isLoading || statusQuery.isLoading
              ? 'Loading status…'
              : hasModel
                ? <>Agent ready — model <code className={s.codeInline}>{status?.model}</code></>
                : <>Degraded — {status?.degraded_reason ?? 'no provider configured'}. Set an API key below.</>}
          </span>
        </div>

        {error && (
          <div className={s.errorBar}>
            <Icon icon={AlertTriangle} size={14} />
            <span>{error}</span>
          </div>
        )}

        {/* Permission mode 快捷控件（始终可见，高频操作）—— 4 个按钮文案 auto/prompt/deny/plan。 */}
        <div className={s.quickPerm}>
          <span className={s.quickPermLabel}>Permission mode:</span>
          <div className={s.quickPermBtns}>
            {(['auto', 'prompt', 'deny', 'plan'] as PermissionMode[]).map((m) => (
              <button
                key={m}
                className={s.quickPermBtn}
                onClick={() => onPermission(m)}
                disabled={permMutation.isPending}
              >
                {m}
              </button>
            ))}
          </div>
        </div>

        {section === 'provider' && (
          <section>
            <h3 className={s.sectionTitle}>Provider</h3>
            <p className={s.sectionDesc}>
              Configure the active provider and credentials. Settings persist to{' '}
              <code className={s.codeInline}>~/.reflect/config.toml</code>.
            </p>

            <div className={s.fieldRow}>
              <label className={s.fieldLabel}>Active provider</label>
              <select
                value={fields.activeProvider}
                onChange={(e) => setRawToml(buildToml({ activeProvider: e.target.value }))}
                className={s.nativeSelect}
              >
                <option value="anthropic">Anthropic</option>
                <option value="openai">OpenAI</option>
                <option value="ollama">Ollama (local)</option>
              </select>
            </div>

            <ProviderCard
              name="Anthropic"
              apiKey={fields.anthropicKey}
              model={fields.anthropicModel}
              apiKeyPlaceholder="sk-ant-..."
              modelPlaceholder="claude-3-5-sonnet-latest"
              showKey={showKeys['anthropic'] ?? false}
              onToggleKey={() => setShowKeys((p) => ({ ...p, anthropic: !p.anthropic }))}
              onApiKey={(v) => setRawToml(buildToml({ anthropicKey: v }))}
              onModel={(v) => setRawToml(buildToml({ anthropicModel: v }))}
            />
            <ProviderCard
              name="OpenAI"
              apiKey={fields.openaiKey}
              model={fields.openaiModel}
              apiKeyPlaceholder="sk-..."
              modelPlaceholder="gpt-4o"
              showKey={showKeys['openai'] ?? false}
              onToggleKey={() => setShowKeys((p) => ({ ...p, openai: !p.openai }))}
              onApiKey={(v) => setRawToml(buildToml({ openaiKey: v }))}
              onModel={(v) => setRawToml(buildToml({ openaiModel: v }))}
            />

            <p className={s.hint}>
              Env vars <code className={s.codeInline}>ANTHROPIC_API_KEY</code> /{' '}
              <code className={s.codeInline}>OPENAI_API_KEY</code> /{' '}
              <code className={s.codeInline}>OLLAMA_HOST</code> also work without writing to config.
            </p>
          </section>
        )}

        {section === 'permissions' && (
          <section>
            <h3 className={s.sectionTitle}>Permissions</h3>
            <p className={s.sectionDesc}>
              Control how the agent asks before running tools. Use the quick switch above to change
              mode for the current session.
            </p>
            <div className={s.permGrid}>
              <PermCard mode="auto" onClick={() => onPermission('auto')} />
              <PermCard mode="prompt" onClick={() => onPermission('prompt')} />
              <PermCard mode="deny" onClick={() => onPermission('deny')} />
              <PermCard mode="plan" onClick={() => onPermission('plan')} />
            </div>
          </section>
        )}

        {section === 'advanced' && (
          <section>
            <h3 className={s.sectionTitle}>Advanced (raw TOML)</h3>
            <p className={s.sectionDesc}>
              Edit <code className={s.codeInline}>[mcp_servers.&lt;name&gt;]</code> /{' '}
              <code className={s.codeInline}>[lsp_server.&lt;name&gt;]</code> sections. A restart is
              required to spawn new MCP/LSP servers (hot-reload coming later).
            </p>
            <Textarea
              value={rawToml}
              onChange={(e) => setRawToml(e.target.value)}
              className={s.rawEditor}
              spellCheck={false}
            />
          </section>
        )}

        <div className={s.saveBar}>
          <Button
            variant="primary"
            onClick={onSave}
            disabled={saveMutation.isPending || configQuery.isLoading}
            loading={saveMutation.isPending}
          >
            {saved ? 'Saved ✓' : 'Save to ~/.reflect/config.toml'}
          </Button>
        </div>
      </main>
    </div>
  );
}

// ====== 子组件 ======

function NavBtn({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button className={s.navBtn} data-active={active || undefined} onClick={onClick}>
      {children}
    </button>
  );
}

function ProviderCard({
  name,
  apiKey,
  model,
  apiKeyPlaceholder,
  modelPlaceholder,
  showKey,
  onToggleKey,
  onApiKey,
  onModel,
}: {
  name: string;
  apiKey: string;
  model: string;
  apiKeyPlaceholder: string;
  modelPlaceholder: string;
  showKey: boolean;
  onToggleKey: () => void;
  onApiKey: (v: string) => void;
  onModel: (v: string) => void;
}) {
  return (
    <div className={s.providerCard}>
      <div className={s.providerHeader}>
        <span className={s.providerName}>{name}</span>
        {apiKey && <Badge variant="success" dot>configured</Badge>}
      </div>
      <div className={s.providerFields}>
        <div>
          <label className={s.fieldLabel}>API key</label>
          <Input
            type={showKey ? 'text' : 'password'}
            value={apiKey}
            placeholder={apiKeyPlaceholder}
            onChange={(e) => onApiKey(e.target.value)}
            trailing={
              <IconButton label={showKey ? 'Hide key' : 'Show key'} size="sm" onClick={onToggleKey}>
                <Icon icon={showKey ? EyeOff : Eye} size={13} />
              </IconButton>
            }
          />
        </div>
        <div>
          <label className={s.fieldLabel}>Model</label>
          <Input value={model} placeholder={modelPlaceholder} onChange={(e) => onModel(e.target.value)} />
        </div>
      </div>
    </div>
  );
}

function PermCard({ mode, onClick }: { mode: PermissionMode; onClick: () => void }) {
  const variant = mode === 'auto' ? 'success' : mode === 'deny' ? 'danger' : mode === 'plan' ? 'info' : 'warning';
  const desc: Record<PermissionMode, string> = {
    auto: 'Run tools without asking. Fastest, least safe.',
    prompt: 'Ask before each tool call. Recommended.',
    deny: 'Block all tool execution. Read-only chat.',
    plan: 'Only plan, never execute. Explore safely.',
  };
  return (
    <button className={s.permCard} data-variant={variant} onClick={onClick}>
      <div className={s.permCardHeader}>
        <Badge variant={variant} solid>{mode}</Badge>
      </div>
      <p className={s.permDesc}>{desc[mode]}</p>
    </button>
  );
}

// ====== TOML 字段解析 / 合并（保留原逻辑） ======

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

function readField(toml: string, key: string, fallback: string, section?: string): string {
  const lines = toml.split('\n');
  let inSection = !section;
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
      if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) {
        v = v.slice(1, -1);
      }
      return v;
    }
  }
  return fallback;
}

function mergeFields(original: string, f: Fields): string {
  let out = original;
  out = ensureActiveProvider(out, f.activeProvider);
  out = setInSection(out, 'anthropic', 'api_key', f.anthropicKey);
  out = setInSection(out, 'anthropic', 'model', f.anthropicModel);
  out = setInSection(out, 'openai', 'api_key', f.openaiKey);
  out = setInSection(out, 'openai', 'model', f.openaiModel);
  return out;
}

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
    lines.push('');
    lines.push('[active]');
    lines.push(`provider = "${provider}"`);
  }
  return lines.join('\n');
}

function setInSection(toml: string, section: string, key: string, value: string): string {
  const lines = toml.split('\n');
  if (value === '') {
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
    lines.push('');
    lines.push(`[${section}]`);
    lines.push(`${key} = "${value}"`);
    return lines.join('\n');
  }
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
