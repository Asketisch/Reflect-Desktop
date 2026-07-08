/**
 * M3.x Design System —— 组件库参考（catalog）。
 *
 * 展示 primitives + token。
 */

import { Button } from './primitives/Button';
import { Toast } from './primitives/Toast';
import { ContextRing } from './primitives/ContextRing';
import { KeyHint } from './primitives/KeyHint';
import { TOAST_COLORS } from './utils/toast';
import { shortcutLabel, platformLabel } from './utils/keyHints';

export function DesignSystemView() {
  return (
    <div style={{ padding: 32, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Design System</h1>

      <section style={{ marginBottom: 32 }} data-section="colors">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Colors</h2>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {Object.entries(TOAST_COLORS).map(([k, c]) => (
            <div key={k} style={{ width: 80 }}>
              <div style={{ height: 40, background: c.bg, border: `1px solid ${c.border}`, borderRadius: 6 }} />
              <div style={{ fontSize: 11, marginTop: 4 }}>{k}</div>
            </div>
          ))}
        </div>
      </section>

      <section style={{ marginBottom: 32 }} data-section="typography">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Typography</h2>
        <div style={{ fontSize: 28, fontWeight: 700, marginBottom: 4 }}>Heading 1</div>
        <div style={{ fontSize: 22, fontWeight: 600, marginBottom: 4 }}>Heading 2</div>
        <div style={{ fontSize: 16, fontWeight: 500, marginBottom: 4 }}>Heading 3</div>
        <div style={{ fontSize: 14, color: '#666' }}>Body text — 14px, regular weight</div>
      </section>

      <section style={{ marginBottom: 32 }} data-section="buttons">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Buttons</h2>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center' }}>
          <Button variant="primary">Primary</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="danger">Danger</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="primary" size="sm">Small</Button>
          <Button variant="primary" disabled>Disabled</Button>
        </div>
      </section>

      <section style={{ marginBottom: 32 }} data-section="context-ring">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Context Ring</h2>
        <ContextRing
          segments={[
            { label: 'tools', value: 0.15, color: '#3b82f6' },
            { label: 'system', value: 0.1, color: '#22c55e' },
            { label: 'skills', value: 0.05, color: '#f59e0b' },
            { label: 'messages', value: 0.3, color: '#8b5cf6' },
          ]}
          label="60% used"
        />
      </section>

      <section style={{ marginBottom: 32 }} data-section="toast">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Toast</h2>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, maxWidth: 400 }}>
          <Toast kind="info" message="Configuration reloaded" onDismiss={() => {}} />
          <Toast kind="success" message="Session renamed" onDismiss={() => {}} />
          <Toast kind="warning" message="Context nearly full" onDismiss={() => {}} />
          <Toast kind="error" message="Failed to fetch sessions" onDismiss={() => {}} />
        </div>
      </section>

      <section style={{ marginBottom: 32 }} data-section="key-hints">
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Key Hints</h2>
        <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
          <KeyHint combo="Cmd+Enter" platform="macos" label="Send" />
          <KeyHint combo="Cmd+." platform="macos" label="Interrupt" />
          <KeyHint combo="Ctrl+Enter" platform="other" label="Send" />
          <KeyHint combo="Esc" platform="other" label="Cancel" />
        </div>
        <div style={{ marginTop: 8, fontSize: 12, color: '#64748b' }}>
          macOS Cmd: {platformLabel('macos').cmd} · Other Ctrl: {platformLabel('other').cmd} ·
          Combo example: {shortcutLabel('Cmd+Shift+P', 'macos')}
        </div>
      </section>
    </div>
  );
}