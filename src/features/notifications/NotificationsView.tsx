/**
 * M3.x Notifications —— 通知中心。
 *
 * - 列出最近的系统通知
 * - 清除 / 标记已读
 * - M3.x 扩展:persistent notification store + toast popup
 */

import { useState } from 'react';

interface Notification {
  id: string;
  title: string;
  body: string;
  time: string;
  read: boolean;
}

const STUB_NOTIFICATIONS: Notification[] = [
  { id: 'n1', title: 'Agent idle', body: 'Reflect agent is waiting for input.', time: new Date(Date.now() - 60000).toISOString(), read: false },
  { id: 'n2', title: 'Turn complete', body: 'Session abc123 finished successfully.', time: new Date(Date.now() - 300000).toISOString(), read: true },
  { id: 'n3', title: 'Update available', body: 'Reflect Desktop 0.2.0 is available.', time: new Date(Date.now() - 86400000).toISOString(), read: false },
];

export function NotificationsView() {
  const [notifications, setNotifications] = useState<Notification[]>(STUB_NOTIFICATIONS);

  const unread = notifications.filter((n) => !n.read).length;

  const markRead = (id: string) => {
    setNotifications((prev) => prev.map((n) => (n.id === id ? { ...n, read: true } : n)));
  };

  const clearAll = () => {
    setNotifications([]);
  };

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16 }}>
        <h1 style={{ fontSize: 22, margin: 0, flex: 1 }}>Notifications</h1>
        {unread > 0 && (
          <span style={{ padding: '2px 8px', background: '#3b82f6', color: 'white', borderRadius: 12, fontSize: 11 }}>
            {unread}
          </span>
        )}
        <button onClick={clearAll} style={{ marginLeft: 12, padding: '4px 12px', cursor: 'pointer' }}>
          Clear all
        </button>
      </div>

      {notifications.length === 0 && (
        <p style={{ color: '#888' }}>No notifications.</p>
      )}

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {notifications.map((n) => (
          <li
            key={n.id}
            onClick={() => markRead(n.id)}
            style={{
              padding: 12,
              border: '1px solid #e2e8f0',
              borderRadius: 8,
              marginBottom: 8,
              background: n.read ? 'white' : '#f0f9ff',
              cursor: 'pointer',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
              <strong style={{ fontSize: 13 }}>{n.title}</strong>
              {!n.read && <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#3b82f6', display: 'inline-block' }} />}
            </div>
            <p style={{ margin: 0, fontSize: 13, color: '#666' }}>{n.body}</p>
            <span style={{ fontSize: 11, color: '#aaa', marginTop: 4, display: 'block' }}>
              {new Date(n.time).toLocaleString()}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
