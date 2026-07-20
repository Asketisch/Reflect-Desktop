/**
 * Apps —— App 集成（stub，EmptyState 美化）。
 */
import { useState } from 'react';
import { AppWindow, Plug, Plug2 } from 'lucide-react';
import type { ComponentType } from 'react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Button, Icon } from '@/features/design-system';
import s from './AppsView.module.css';

interface App {
  id: string;
  name: string;
  icon: ComponentType;
  connected: boolean;
  permissions: string[];
}

const STUB_APPS: App[] = [
  { id: 'vscode', name: 'VS Code', icon: AppWindow, connected: true, permissions: ['read_files', 'exec_command'] },
  { id: 'jetbrains', name: 'JetBrains', icon: AppWindow, connected: false, permissions: [] },
  { id: 'cursor', name: 'Cursor', icon: AppWindow, connected: false, permissions: [] },
];

export function AppsView() {
  const [apps, setApps] = useState<App[]>(STUB_APPS);

  const toggle = (id: string) => {
    setApps((prev) => prev.map((a) => (a.id === id ? { ...a, connected: !a.connected } : a)));
  };

  return (
    <PageShell
      icon={AppWindow}
      title="Apps"
      subtitle={
        <>
          Connect IDEs and external tools. <Badge variant="warning">sample data</Badge>
        </>
      }
      width="md"
    >
      <div className={s.list}>
        {apps.map((a) => (
          <Card key={a.id} level="outlined" padding="md" className={s.appRow}>
            <div className={s.appIcon}>
              <Icon icon={a.icon} size={20} />
            </div>
            <div className={s.appBody}>
              <div className={s.appNameRow}>
                <span className={s.appName}>{a.name}</span>
                {a.connected && <Badge variant="success" dot>connected</Badge>}
              </div>
              {a.permissions.length > 0 ? (
                <div className={s.permList}>
                  {a.permissions.map((p) => (
                    <code key={p} className={s.perm}>{p}</code>
                  ))}
                </div>
              ) : (
                <div className={s.appDesc}>Not connected</div>
              )}
            </div>
            <Button
              variant={a.connected ? 'danger' : 'primary'}
              size="sm"
              onClick={() => toggle(a.id)}
              leftIcon={<Icon icon={a.connected ? Plug2 : Plug} size={13} />}
            >
              {a.connected ? 'Disconnect' : 'Connect'}
            </Button>
          </Card>
        ))}
      </div>
    </PageShell>
  );
}
