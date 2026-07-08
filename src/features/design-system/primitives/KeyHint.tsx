/**
 * KeyHint —— 单个键盘快捷键标签。
 */
import { shortcutLabel, detectPlatform, type Platform } from '../utils/keyHints';

export interface KeyHintProps {
  combo: string;
  platform?: Platform;
  label?: string;
}

export function KeyHint({ combo, platform, label }: KeyHintProps) {
  const p = platform ?? detectPlatform();
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontSize: 11, color: '#64748b' }}>
      {label && <span>{label}</span>}
      <kbd
        style={{
          fontFamily: 'ui-monospace, Menlo, monospace',
          background: '#f1f5f9',
          border: '1px solid #e2e8f0',
          borderRadius: 4,
          padding: '1px 6px',
          fontSize: 11,
        }}
      >
        {shortcutLabel(combo, p)}
      </kbd>
    </span>
  );
}