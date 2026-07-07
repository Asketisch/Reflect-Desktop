/**
 * M3.x Dictation —— 语音输入。
 *
 * M3.x 扩展:Web Speech API / macOS Speech Recognition。
 */

export function DictationView() {
  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Dictation</h1>
      <p style={{ color: '#888', fontSize: 13, marginBottom: 16 }}>
        M3.x: voice input via Web Speech API or native speech recognition.
      </p>
      <div style={{ padding: 40, textAlign: 'center', color: '#aaa', border: '1px dashed #e2e8f0', borderRadius: 8 }}>
        🎤 Voice input coming in M3.x
      </div>
    </div>
  );
}
