/**
 * Inspector —— 右侧可折叠面板。
 *
 * 阶段 2：展示运行时状态（MCP/LSP server、pending 交互计数、最近错误）。
 * 阶段 3：token usage（最近一次 token_count 快照：input/output/cached/cache_write/total/cost）。
 *   - 后端每轮 LLM 调用后 emit `token_count` 事件,reducer 写入 `state.tokens`。
 *   - 这里只读 + 渲染,不做累计(后端 model_call 节点已做 session 级累计)。
 */
import { Server, Cpu, AlertTriangle, MessageCircleQuestion, Coins } from 'lucide-react';
import { Icon, Badge, Tooltip } from '@/features/design-system';
import { useAgentStore } from '@/stores/agentStore';
import { useI18n } from '@/utils/i18n';
import s from './Inspector.module.css';

export function Inspector() {
  const mcpServers = useAgentStore((st) => st.mcpServers);
  const lspServers = useAgentStore((st) => st.lspServers);
  const pendingApprovals = useAgentStore((st) => st.pendingApprovals);
  const pendingQuestions = useAgentStore((st) => st.pendingQuestions);
  const pendingAskUser = useAgentStore((st) => st.pendingAskUser);
  const lastError = useAgentStore((st) => st.lastError);
  const clearError = useAgentStore((st) => st.clearError);
  const tokens = useAgentStore((st) => st.tokens);
  const { t } = useI18n();

  const pendingTotal = pendingApprovals.length + pendingQuestions.length + pendingAskUser.length;

  return (
    <div className={s.root}>
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

      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Server} size={14} />
          {t('inspector.mcpServers')}
          {mcpServers.length > 0 && <Badge variant="neutral">{mcpServers.length}</Badge>}
        </h3>
        {mcpServers.length === 0 ? (
          <p className={s.empty}>{t('inspector.mcpEmpty')}</p>
        ) : (
          <ul className={s.list}>
            {mcpServers.map((m) => (
              <li key={m.name} className={s.row}>
                <span className={s.name}>{m.name}</span>
                <span className={s.dot} data-status={m.status} title={m.detail ?? m.status} />
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Cpu} size={14} />
          {t('inspector.lspServers')}
          {lspServers.length > 0 && <Badge variant="neutral">{lspServers.length}</Badge>}
        </h3>
        {lspServers.length === 0 ? (
          <p className={s.empty}>{t('inspector.lspEmpty')}</p>
        ) : (
          <ul className={s.list}>
            {lspServers.map((l) => (
              <li key={l.name} className={s.row}>
                <span className={s.name}>{l.name}</span>
                <span className={s.dot} data-status={l.status} title={l.detail ?? l.status} />
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Coins} size={14} />
          {t('inspector.tokenUsage')}
          {tokens && <Badge variant="neutral">{tokens.total.toLocaleString()}</Badge>}
        </h3>
        {!tokens ? (
          <p className={s.empty}>{t('inspector.tokenEmpty')}</p>
        ) : (
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
            <li className={s.row}>
              <span className={`${s.name} ${s.strong}`}>{t('inspector.tokenTotal')}</span>
              <span className={`${s.value} ${s.strong}`}>
                {tokens.total.toLocaleString()}
              </span>
            </li>
            {tokens.cost != null && (
              <li className={s.row}>
                <span className={s.name}>{t('inspector.tokenCost')}</span>
                <span className={s.value}>${tokens.cost.toFixed(4)}</span>
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
        )}
      </section>

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
    </div>
  );
}
