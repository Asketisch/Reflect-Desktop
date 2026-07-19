/**
 * About —— 阶段 5b:用真实命令(ping + agent_status)替换 phantom invoke。
 */
import { useQuery } from '@tanstack/react-query';
import { ping, reflect_agent_status } from '@/utils/tauri';

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
    <div style={{ padding: 32, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>About Reflect Desktop</h1>

      <div style={{ marginBottom: 24 }}>
        <Row label="Version" value={version} />
        <Row label="Build" value={new Date().toLocaleDateString()} />
        {status && (
          <>
            <Row label="Model" value={status.model} />
            <Row label="Workspace" value={status.workspace} />
            <Row
              label="Status"
              value={status.has_model ? '✓ ready' : '⚠ degraded (set API key in Settings)'}
            />
          </>
        )}
      </div>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 16, marginBottom: 8 }}>Reflect Agent</h2>
        <p style={{ fontSize: 13, color: '#666' }}>
          Standalone desktop GUI for Reflect Agent — an AI coding agent that helps you write,
          review, and refactor code.
        </p>
      </section>

      <section style={{ marginBottom: 24 }}>
        <h2 style={{ fontSize: 16, marginBottom: 8 }}>Open Source</h2>
        <p style={{ fontSize: 13, color: '#666' }}>
          Built with Tauri 2, React 19, TanStack Router, and the Reflect Agent core.
        </p>
      </section>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <p style={{ margin: '4px 0', fontSize: 14 }}>
      <strong>{label}:</strong> {value}
    </p>
  );
}
