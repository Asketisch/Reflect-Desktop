/**
 * Inspector —— 右侧可折叠面板（v1.x P2：概览 / 文件 / 改动 三 tab，对标 Reasonix）。
 *
 * - **概览**：上下文窗口仪表（占比条 + 压缩阈值提示 + 压缩次数/节省 tokens
 *   聚合统计）+ Token 构成堆叠条
 *   （输入/缓存命中/缓存写入/输出）+ 会话指标（turn 数、累计 token、费用、
 *   命中 provider/凭证）+ 运行时状态（pending 交互、MCP/LSP、最近错误）。
 * - **文件**：当前工作区文件树（depth 2，复用 FilesView 的 FileTree）。
 * - **改动**：工作区 git diff（`reflect_git_diff` + DiffViewer，同 ChatView 分屏）。
 *
 * 数据获取用 useEffect + useState（Inspector 可能脱离 QueryClientProvider
 * 挂载，如独立测试），失败静默为空态。
 */
import { useEffect, useState } from 'react';
import { Server, Cpu, AlertTriangle, MessageCircleQuestion, Coins, Folder, GitBranch, Gauge } from 'lucide-react';
import { Icon, Badge, Tooltip } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import { reflect_list_dir, type ReflectDirEntry } from '@/utils/commands/files';
import { reflect_git_diff } from '@/utils/commands/git';
import { FileTree } from '@/features/files/FileTree';
import { DiffViewer } from '@/features/git/DiffViewer';
import { useContextRatio } from './useContextRatio';
import s from './Inspector.module.css';

type InspectorTab = 'overview' | 'files' | 'changes';

export function Inspector() {
  const { t } = useI18n();
  const [tab, setTab] = useState<InspectorTab>('overview');

  return (
    <div className={s.root}>
      <div className={s.tabs} role="tablist" aria-label={t('inspector.title')} data-testid="inspector-tabs">
        <button type="button" role="tab" aria-selected={tab === 'overview'} className={s.tab} data-active={tab === 'overview' || undefined} onClick={() => setTab('overview')} data-testid="inspector-tab-overview">
          <Icon icon={Gauge} size={12} /> {t('inspector.tab.overview')}
        </button>
        <button type="button" role="tab" aria-selected={tab === 'files'} className={s.tab} data-active={tab === 'files' || undefined} onClick={() => setTab('files')} data-testid="inspector-tab-files">
          <Icon icon={Folder} size={12} /> {t('inspector.tab.files')}
        </button>
        <button type="button" role="tab" aria-selected={tab === 'changes'} className={s.tab} data-active={tab === 'changes' || undefined} onClick={() => setTab('changes')} data-testid="inspector-tab-changes">
          <Icon icon={GitBranch} size={12} /> {t('inspector.tab.changes')}
        </button>
      </div>
      {tab === 'overview' && <OverviewTab />}
      {tab === 'files' && <FilesTab />}
      {tab === 'changes' && <ChangesTab />}
    </div>
  );
}

// ====== 概览 ======

function OverviewTab() {
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const pendingApprovals = useAgentStore((st) => st.pendingApprovals);
  const pendingQuestions = useAgentStore((st) => st.pendingQuestions);
  const pendingAskUser = useAgentStore((st) => st.pendingAskUser);
  const lastError = useAgentStore((st) => st.lastError);
  const clearError = useAgentStore((st) => st.clearError);
  const tokens = useAgentStore((st) => st.tokens);
  const turnCount = useAgentStore((st) => st.turns.length);
  const compactions = useAgentStore((st) => st.compactions);
  const ctx = useContextRatio();
  const { t } = useI18n();

  const pendingTotal = pendingApprovals.length + pendingQuestions.length + pendingAskUser.length;

  return (
    <>
      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Gauge} size={14} />
          {t('inspector.contextGauge')}
        </h3>
        {ctx.pct === null ? (
          <p className={s.empty}>{t('inspector.contextEmpty')}</p>
        ) : (
          <div data-testid="inspector-context-gauge">
            <div className={s.gaugeRow}>
              <span className={s.gaugeBadge} data-warn={ctx.warn || undefined}>{ctx.pct}%</span>
              <span className={s.value}>
                {ctx.usedTokens.toLocaleString()} / {(ctx.windowSize ?? 0).toLocaleString()}
              </span>
            </div>
            <div className={s.gaugeBar} role="img" aria-label={`${ctx.pct}%`}>
              <span className={s.gaugeFill} style={{ width: `${Math.max(2, ctx.pct)}%` }} data-warn={ctx.warn || undefined} />
              {/* 压缩阈值提示线（80%，对标 Reasonix「压缩 800000」刻度） */}
              <span className={s.gaugeThreshold} style={{ left: '80%' }} title={t('composer.contextNearFull', { pct: '80' })} />
            </div>
            {ctx.warn && <p className={s.gaugeHint}>{t('composer.contextNearFull', { pct: String(ctx.pct) })}</p>}
          </div>
        )}
        {/* 压缩聚合统计（context_compacted / 历史 compaction record）。
            hover 展示最近一次明细，如 `smart_prune: 0 msgs (7731 → 5344 tokens)`。 */}
        {compactions.count > 0 && (
          <ul className={s.list} data-testid="inspector-compactions" title={compactions.last ?? undefined}>
            <li className={s.row}>
              <span className={s.name}>{t('inspector.compactions')}</span>
              <span className={s.value}>{compactions.count}</span>
            </li>
            {compactions.tokensSaved > 0 && (
              <li className={s.row}>
                <span className={s.name}>{t('inspector.compactionsSaved')}</span>
                <span className={s.value}>{compactions.tokensSaved.toLocaleString()}</span>
              </li>
            )}
          </ul>
        )}
      </section>

      {tokens && (
        <section className={s.section}>
          <h3 className={s.sectionTitle}>
            <Icon icon={Coins} size={14} />
            {t('inspector.tokenUsage')}
            <Badge variant="neutral">{tokens.total.toLocaleString()}</Badge>
          </h3>
          <TokenComposition />
          <ul className={s.list}>
            <li className={s.row}>
              <span className={s.name}>{t('inspector.tokenInput')}</span>
              <span className={s.value}>{tokens.input.toLocaleString()}</span>
            </li>
            <li className={s.row}>
              <span className={s.name}>{t('inspector.tokenOutput')}</span>
              <span className={s.value}>{tokens.output.toLocaleString()}</span>
            </li>
            <li className={s.row}>
              <Tooltip label={t('inspector.tokenCachedHint')} side="left">
                <span className={s.name}>{t('inspector.tokenCached')}</span>
              </Tooltip>
              <span className={s.value}>{tokens.cached.toLocaleString()}</span>
            </li>
            {tokens.cacheWrite > 0 && (
              <li className={s.row}>
                <Tooltip label={t('inspector.tokenCacheWriteHint')} side="left">
                  <span className={s.name}>{t('inspector.tokenCacheWrite')}</span>
                </Tooltip>
                <span className={s.value}>{tokens.cacheWrite.toLocaleString()}</span>
              </li>
            )}
            {tokens.provider && (
              <li className={s.row}>
                <span className={s.name}>{t('inspector.tokenProvider')}</span>
                <span className={s.value}>{tokens.provider}</span>
              </li>
            )}
            {tokens.credentialLabel && (
              <li className={s.row}>
                <span className={s.name}>{t('inspector.tokenCredential')}</span>
                <span className={s.value}>{tokens.credentialLabel}</span>
              </li>
            )}
          </ul>
        </section>
      )}

      <section className={s.section}>
        <h3 className={s.sectionTitle}>{t('inspector.sessionMetrics')}</h3>
        <ul className={s.list}>
          <li className={s.row}>
            <span className={s.name}>{t('inspector.metricTurns')}</span>
            <span className={s.value} data-testid="inspector-turn-count">{turnCount}</span>
          </li>
          <li className={s.row}>
            <span className={s.name}>{t('inspector.metricTotalTokens')}</span>
            <span className={s.value}>{(tokens?.total ?? 0).toLocaleString()}</span>
          </li>
        </ul>
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={MessageCircleQuestion} size={14} />
          {t('inspector.pending')}
          {pendingTotal > 0 && <Badge variant="warning" solid>{pendingTotal}</Badge>}
        </h3>
        {pendingTotal === 0 ? (
          <p className={s.empty}>{t('inspector.pendingEmpty')}</p>
        ) : (
          <ul className={s.list}>
            {pendingApprovals.length > 0 && (
              <li className={s.row}>
                <span>{t('inspector.pendingApprovals')}</span>
                <Badge variant="warning">{pendingApprovals.length}</Badge>
              </li>
            )}
            {pendingQuestions.length > 0 && (
              <li className={s.row}>
                <span>{t('inspector.pendingQuestions')}</span>
                <Badge variant="warning">{pendingQuestions.length}</Badge>
              </li>
            )}
            {pendingAskUser.length > 0 && (
              <li className={s.row}>
                <span>{t('inspector.pendingInputs')}</span>
                <Badge variant="warning">{pendingAskUser.length}</Badge>
              </li>
            )}
          </ul>
        )}
      </section>

      <ServersSection
        icon={Server}
        title={t('inspector.mcpServers')}
        count={mcpServers.length}
        emptyText={t('inspector.mcpEmpty')}
        entries={mcpServers.map((m) => ({ name: m.name, status: m.status, detail: m.detail }))}
      />
      <ServersSection
        icon={Cpu}
        title={t('inspector.lspServers')}
        count={lspServers.length}
        emptyText={t('inspector.lspEmpty')}
        entries={lspServers.map((l) => ({ name: l.name, status: l.status, detail: l.detail }))}
      />

      {lastError && (
        <section className={s.section}>
          <h3 className={s.sectionTitle}>
            <Icon icon={AlertTriangle} size={14} />
            {t('inspector.lastError')}
          </h3>
          <div className={s.errorBox}>
            <pre className={s.errorText}>{lastError}</pre>
            <button className={s.dismiss} onClick={clearError}>
              {t('inspector.dismiss')}
            </button>
          </div>
        </section>
      )}
    </>
  );
}

/** Token 构成堆叠条（输入 / 缓存命中 / 缓存写入 / 输出），对标 Reasonix「Token 构成」。 */
function TokenComposition() {
  const tokens = useAgentStore((st) => st.tokens);
  const { t } = useI18n();
  if (!tokens) return null;
  const parts = [
    { key: 'input', value: Math.max(0, tokens.input), color: 'var(--accent)', label: t('inspector.comp.input') },
    { key: 'cached', value: tokens.cached, color: 'var(--info, #60a5fa)', label: t('inspector.comp.cached') },
    { key: 'cacheWrite', value: tokens.cacheWrite, color: 'var(--warning)', label: t('inspector.comp.cacheWrite') },
    { key: 'output', value: tokens.output, color: 'var(--success, #4ade80)', label: t('inspector.comp.output') },
  ];
  const total = parts.reduce((sum, p) => sum + p.value, 0);
  if (total <= 0) return null;
  const present = parts.filter((p) => p.value > 0);
  return (
    <div className={s.comp} data-testid="inspector-token-composition">
      <div className={s.compBar}>
        {present.map((p) => (
          <Tooltip key={p.key} label={`${p.label}: ${p.value.toLocaleString()}`} side="left">
            <span className={s.compSeg} style={{ width: `${(p.value / total) * 100}%`, background: p.color }} />
          </Tooltip>
        ))}
      </div>
      <div className={s.compLegend}>
        {present.map((p) => (
          <span key={p.key} className={s.compLegendItem}>
            <span className={s.compDot} style={{ background: p.color }} />
            {p.label}
          </span>
        ))}
      </div>
    </div>
  );
}

// ====== 服务器状态（MCP / LSP 共用渲染） ======

function ServersSection({ icon: SectionIcon, title, count, emptyText, entries }: {
  icon: typeof Server;
  title: string;
  count: number;
  emptyText: string;
  entries: { name: string; status: string; detail?: string }[];
}) {
  return (
    <section className={s.section}>
      <h3 className={s.sectionTitle}>
        <Icon icon={SectionIcon} size={14} />
        {title}
        {count > 0 && <Badge variant="neutral">{count}</Badge>}
      </h3>
      {count === 0 ? (
        <p className={s.empty}>{emptyText}</p>
      ) : (
        <ul className={s.list}>
          {entries.map((m) => (
            <li key={m.name} className={s.row}>
              <span className={s.name}>{m.name}</span>
              <span className={s.dot} data-status={m.status} title={m.detail ?? m.status} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

// ====== 文件 ======

function FilesTab() {
  const { t } = useI18n();
  const [entries, setEntries] = useState<ReflectDirEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    reflect_list_dir(null, 2)
      .then((listing) => {
        if (!cancelled) setEntries(listing.entries ?? []);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className={s.filesWrap} data-testid="inspector-files">
      {error ? (
        <p className={s.empty}>{error}</p>
      ) : entries.length === 0 ? (
        <p className={s.empty}>{t('inspector.filesEmpty')}</p>
      ) : (
        <FileTree entries={entries} />
      )}
    </div>
  );
}

// ====== 改动 ======

function ChangesTab() {
  const { t } = useI18n();
  const [diff, setDiff] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    reflect_git_diff(false)
      .then((d) => {
        if (!cancelled) setDiff(d);
      })
      .catch(() => {
        if (!cancelled) setDiff('');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className={s.changesWrap} data-testid="inspector-changes">
      <pre className={s.changesPre}>
        <DiffViewer diff={diff ?? ''} emptyMessage={t('chat.diffEmpty')} />
      </pre>
    </div>
  );
}
