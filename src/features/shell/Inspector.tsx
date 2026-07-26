/**
 * Inspector —— 右侧可折叠面板。
 *
 * 阶段 2：展示运行时状态（MCP/LSP server、pending 交互计数、最近错误）。
 * 阶段 3 会扩展：context 用量环、token cost、工具调用详情。
 */
import { Server, Cpu, AlertTriangle, MessageCircleQuestion } from 'lucide-react';
import { Icon, Badge } from '@/features/design-system';
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
