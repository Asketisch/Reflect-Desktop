/**
 * Mobile —— 移动伴侣视图（stub，EmptyState 占位）。
 */
import { Smartphone } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, EmptyState } from '@/features/design-system';

export function MobileView() {
  return (
    <PageShell
      icon={Smartphone}
      title="Mobile"
      subtitle={
        <>
          Mobile companion view with touch-optimized layout. <Badge variant="info">coming soon</Badge>
        </>
      }
      width="md"
    >
      <Card level="flat" padding="none">
        <EmptyState
          size="lg"
          icon={<Icon icon={Smartphone} size={32} />}
          title="Mobile companion coming soon"
          description="Pair with the desktop app from your phone — responsive layout, touch gestures, and push notifications."
        />
      </Card>
    </PageShell>
  );
}
