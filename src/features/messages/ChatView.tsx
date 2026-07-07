/**
 * M1.5 ChatView —— MessageList + 新 Composer (slash + toolbar)。
 */
import { MessageList } from './MessageList';
import { Composer } from './Composer';

export function ChatView() {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ flex: 1, overflow: 'auto' }}>
        <MessageList />
      </div>
      <Composer />
    </div>
  );
}
