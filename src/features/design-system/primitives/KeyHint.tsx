/**
 * KeyHint —— 单个键盘快捷键标签（CSS Modules 版）。
 */
import { shortcutLabel, detectPlatform, type Platform } from '../utils/keyHints';
import s from './KeyHint.module.css';

export interface KeyHintProps {
  combo: string;
  platform?: Platform;
  label?: string;
}

export function KeyHint({ combo, platform, label }: KeyHintProps) {
  const p = platform ?? detectPlatform();
  return (
    <span className={s.wrap}>
      {label && <span className={s.label}>{label}</span>}
      <kbd className={s.kbd}>{shortcutLabel(combo, p)}</kbd>
    </span>
  );
}
