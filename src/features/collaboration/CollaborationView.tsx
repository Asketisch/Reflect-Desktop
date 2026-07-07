/**
 * M3.x Collaboration —— 多人协作。
 *
 * - 显示协作会话
 * - 在线用户列表
 * - M3.x 扩展:real-time sync + shared sessions
 */

import { useState } from 'react';

interface CollabUser {
  id: string;
  name: string;
  color: string;
  active: boolean;
}

const STUB_USERS: CollabUser[] = [
  { id: 'u1', name: 'You', color: '#3b82f6', active: true },
  { id: 'u2', name: 'Alice', color: '#22c55e', active: false },
];

export function CollaborationView() {
  const [users] = useState<CollabUser[]>(STUB_USERS);

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Collaboration</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: real-time multi-user collaboration. Current session: solo.
      </p>

      <div style={{ display: 'flex', gap: 12 }}>
        {users.map((u) => (
          <div
            key={u.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '8px 12px',
              border: '1px solid #e2e8f0',
              borderRadius: 8,
              opacity: u.active ? 1 : 0.6,
            }}
          >
            <span style={{ width: 12, height: 12, borderRadius: '50%', background: u.color, display: 'inline-block' }} />
            <span style={{ fontSize: 13, fontWeight: u.active ? 600 : 400 }}>{u.name}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
