/**
 * About —— 版本/状态信息（CSS Modules 版）。
 */
import { useQuery } from '@tanstack/react-query';
import { Info, Github, Heart } from 'lucide-react';
import { ping, reflect_agent_status } from '@/utils/tauri';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon } from '@/features/design-system';
import s from './AboutView.module.css';

export function AboutView() {
  const pingQ = useQuery({ queryKey: ['ping'], queryFn: ping, staleTime: Infinity });
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  const version = pingQ.data?.version ?? '—';
  const status = statusQ.data;

  return (
    <PageShell icon={Info} title="About Reflect Desktop" width="md">
      {/* Brand */}
      <Card level="outlined" padding="lg" className={s.brandCard}>
        <div className={s.brandRow}>
          <div className={s.logo}>R</div>
          <div>
            <div className={s.brandName}>Reflect Desktop</div>
            <div className={s.brandTagline}>Standalone desktop GUI for the Reflect Agent.</div>
          </div>
        </div>
      </Card>

      {/* Info rows */}
      <Card level="outlined" padding="md" className={s.infoCard}>
        <Row label="Version" value={<code className={s.mono}>{version}</code>} />
        <Row label="Build date" value={new Date().toLocaleDateString()} />
        {status && (
          <>
            <Row label="Model" value={<code className={s.mono}>{status.model}</code>} />
            <Row label="Workspace" value={<code className={s.mono}>{status.workspace}</code>} />
            <Row
              label="Status"
              value={
                status.has_model ? (
                  <Badge variant="success" dot>ready</Badge>
                ) : (
                  <Badge variant="warning" dot>degraded</Badge>
                )
              }
            />
          </>
        )}
      </Card>

      {/* Sections */}
      <section className={s.section}>
        <h3 className={s.sectionTitle}>Reflect Agent</h3>
        <p className={s.sectionText}>
          An AI coding agent that helps you write, review, and refactor code. Reflect Desktop is the
          native GUI companion.
        </p>
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>Built with</h3>
        <p className={s.sectionText}>
          Tauri 2 · React 19 · TanStack Router/Query · Zustand · Vite. Reflect-Agent core provides the
          protocol and tool runtime.
        </p>
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>Links</h3>
        <div className={s.linkRow}>
          <span className={s.linkItem}>
            <Icon icon={Github} size={14} /> Source &amp; issues
          </span>
          <span className={s.linkItem}>
            <Icon icon={Heart} size={14} /> Open source
          </span>
        </div>
      </section>
    </PageShell>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className={s.row}>
      <span className={s.rowLabel}>{label}</span>
      <span className={s.rowValue}>{value}</span>
    </div>
  );
}
