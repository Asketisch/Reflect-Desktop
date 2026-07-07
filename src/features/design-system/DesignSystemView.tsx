/**
 * M3.x Design System —— 组件库参考。
 *
 * - 展示颜色、字体、间距等设计 token
 * - M3.x 扩展:component playground + storybook
 */

export function DesignSystemView() {
  return (
    <div style={{ padding: 32, maxWidth: 800, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Design System</h1>

      <section style={{ marginBottom: 32 }}>
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Colors</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          {['#3b82f6', '#22c55e', '#ef4444', '#f59e0b', '#8b5cf6', '#ec4899', '#1e293b', '#f1f5f9'].map((c) => (
            <div key={c} style={{ width: 48, height: 48, background: c, borderRadius: 8, border: '1px solid #e2e8f0' }} title={c} />
          ))}
        </div>
      </section>

      <section style={{ marginBottom: 32 }}>
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Typography</h2>
        <div style={{ fontSize: 28, fontWeight: 700, marginBottom: 4 }}>Heading 1</div>
        <div style={{ fontSize: 22, fontWeight: 600, marginBottom: 4 }}>Heading 2</div>
        <div style={{ fontSize: 16, fontWeight: 500, marginBottom: 4 }}>Heading 3</div>
        <div style={{ fontSize: 14, color: '#666' }}>Body text — 14px, regular weight</div>
      </section>

      <section>
        <h2 style={{ fontSize: 16, marginBottom: 12 }}>Buttons</h2>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {['Primary', 'Secondary', 'Danger', 'Ghost'].map((label) => (
            <button
              key={label}
              style={{
                padding: '8px 16px',
                border: '1px solid',
                borderColor: label === 'Primary' ? '#3b82f6' : label === 'Danger' ? '#ef4444' : '#e2e8f0',
                background: label === 'Primary' ? '#3b82f6' : label === 'Danger' ? '#ef4444' : 'white',
                color: ['Primary', 'Danger'].includes(label) ? 'white' : '#1e293b',
                borderRadius: 6,
                cursor: 'pointer',
                fontSize: 13,
              }}
            >
              {label}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}
