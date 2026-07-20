/**
 * Debug —— 调试面板（dev only，CSS Modules 版）。
 */
import { useState, useEffect } from 'react';
import { Bug, Activity, Database } from 'lucide-react';
import { useAgent } from '@/services/agent';
import { PageShell } from '@/features/shell/PageShell';
import { Badge, Icon } from '@/features/design-system';
import s from './DebugView.module.css';

export function DebugView() {
  const { turns, session } = useAgent();
  const [events, setEvents] = useState<string[]>([]);

  useEffect(() => {
    setEvents((prev) => [...prev, `[${new Date().toLocaleTimeString()}] render`]);
  }, [turns.length, session]);

  return (
    <PageShell
      icon={Bug}
      title="Debug"
      subtitle={
        <>
          Dev-only diagnostic panel. <Badge variant="warning">remove in production</Badge>
        </>
      }
      width="lg"
    >
      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Database} size={14} />
          Agent state
        </h3>
        <pre className={s.jsonBlock}>
          {JSON.stringify({ session, turns_count: turns.length }, null, 2)}
        </pre>
      </section>

      <section className={s.section}>
        <h3 className={s.sectionTitle}>
          <Icon icon={Activity} size={14} />
          Event log
        </h3>
        <div className={s.eventLog}>
          {events.map((e, i) => (
            <div key={i} className={s.eventLine}>{e}</div>
          ))}
        </div>
      </section>
    </PageShell>
  );
}
