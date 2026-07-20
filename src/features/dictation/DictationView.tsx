/**
 * Dictation —— 语音输入（stub，EmptyState 占位）。
 */
import { Mic } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, EmptyState } from '@/features/design-system';

export function DictationView() {
  return (
    <PageShell
      icon={Mic}
      title="Dictation"
      subtitle={
        <>
          Voice input via Web Speech API or native speech recognition. <Badge variant="info">coming soon</Badge>
        </>
      }
      width="md"
    >
      <Card level="flat" padding="none">
        <EmptyState
          size="lg"
          icon={<Icon icon={Mic} size={32} />}
          title="Voice input coming soon"
          description="We're integrating Web Speech API and native macOS Speech Recognition so you can dictate prompts hands-free."
        />
      </Card>
    </PageShell>
  );
}
