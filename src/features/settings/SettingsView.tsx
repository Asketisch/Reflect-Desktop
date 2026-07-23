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
 *
 * 实际渲染使用 ConfigForm —— 它覆盖了 ReflectConfig 的 25+ 配置段,
 * 见 vendor/reflect-config/src/schema.rs。Advanced TOML 编辑器仍然是
 * 逃生口,所有未知字段都会保留。
 */
import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, AlertTriangle } from 'lucide-react';
import {
  reflect_get_config,
  reflect_save_config,
  reflect_agent_status,
  reflect_set_permission_mode,
} from '@/utils/tauri';
import { Button, Textarea, Badge, Icon, Select } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { ConfigForm } from './ConfigForm';
import s from './SettingsView.module.css';

type PermissionMode = 'auto' | 'prompt' | 'deny' | 'plan';
type Section = 'provider' | 'permissions' | 'advanced';

export function SettingsView({ onClose }: { onClose?: () => void }) {
  const qc = useQueryClient();
  const [section, setSection] = useState<Section>('provider');
  const [rawToml, setRawToml] = useState('');
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});
  const { t, locale, setLocale } = useI18n();

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

  const onSave = () => saveMutation.mutate(rawToml);
  const onPermission = (mode: PermissionMode) => permMutation.mutate(mode);

  const status = statusQuery.data;
  const hasModel = Boolean(status?.has_model);

  const toggleSecret = (key: string) => setShowSecrets((p) => ({ ...p, [key]: !p[key] }));

  return (
    <div className={s.root}>
      <aside className={s.nav}>
        <h2 className={s.title}>{t('settings.title')}</h2>
        <nav className={s.navList}>
          <NavBtn active={section === 'provider'} onClick={() => setSection('provider')}>
            {t('settings.provider')}
          </NavBtn>
          <NavBtn active={section === 'permissions'} onClick={() => setSection('permissions')}>
            {t('settings.permissions')}
          </NavBtn>
          <NavBtn active={section === 'advanced'} onClick={() => setSection('advanced')}>
            {t('settings.advanced')}
          </NavBtn>
        </nav>
        {onClose && (
          <Button variant="ghost" size="sm" onClick={onClose} className={s.closeBtn}>
            {t('common.close')}
          </Button>
        )}
      </aside>

      <main className={s.content}>
        {/* 状态徽标 —— 契约 'Agent ready' */}
        <div className={s.statusBar} data-ok={hasModel || undefined}>
          <Icon icon={hasModel ? CheckCircle2 : AlertTriangle} size={14} />
          <span>
            {configQuery.isLoading || statusQuery.isLoading
              ? t('settings.loading')
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
            <h3 className={s.sectionTitle}>{t('settings.provider')}</h3>
            <p className={s.sectionDesc}>
              Configure the active provider and credentials. Settings persist to{' '}
              <code className={s.codeInline}>~/.reflect/config.toml</code>.
            </p>

            <div className={s.fieldRow}>
              <label className={s.fieldLabel}>{t('settings.language')}</label>
              <Select value={locale} onChange={(e) => setLocale(e.target.value as 'en' | 'zh-CN')}>
                <option value="en">{t('settings.english')}</option>
                <option value="zh-CN">{t('settings.chinese')}</option>
              </Select>
            </div>

            <h4 className={s.sectionTitle}>{t('settings.configFields')}</h4>
            <p className={s.sectionDesc}>{t('settings.configHelp')}</p>
            <ConfigForm
              rawToml={rawToml}
              onChange={setRawToml}
              showSecrets={showSecrets}
              onToggleSecret={toggleSecret}
            />
          </section>
        )}

        {section === 'permissions' && (
          <section>
            <h3 className={s.sectionTitle}>{t('settings.permissions')}</h3>
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
            <h3 className={s.sectionTitle}>{t('settings.advanced')} (raw TOML)</h3>
            <p className={s.sectionDesc}>
              Edit any section, including ones without a structured form above.
              Validation runs server-side via <code className={s.codeInline}>ReflectConfig::load_from_str</code>.
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
            {saved ? t('settings.saved') : t('settings.save')}
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