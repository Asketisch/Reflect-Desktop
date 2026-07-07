/**
 * M3.x Mobile —— 移动伴侣视图。
 *
 * M3.x 扩展:responsive layout + touch gestures。
 */

export function MobileView() {
  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Mobile</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: mobile companion view with touch-optimized layout。
      </p>
      <div style={{ padding: 40, textAlign: 'center', color: '#aaa', border: '1px dashed #e2e8f0', borderRadius: 8 }}>
        📱 Mobile companion coming in M3.x
      </div>
    </div>
  );
}
