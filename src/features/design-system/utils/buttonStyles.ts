/**
 * Button 样式工具 —— 计算 inline style（避免每次渲染重算）。
 */
export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

export interface ButtonStyleOpts {
  variant: ButtonVariant;
  size?: 'sm' | 'md';
  block?: boolean;
}

const VARIANT_COLORS: Record<ButtonVariant, { bg: string; fg: string; border: string }> = {
  primary: { bg: '#3b82f6', fg: '#ffffff', border: '#3b82f6' },
  secondary: { bg: '#ffffff', fg: '#1e293b', border: '#e2e8f0' },
  danger: { bg: '#ef4444', fg: '#ffffff', border: '#ef4444' },
  ghost: { bg: 'transparent', fg: '#1e293b', border: 'transparent' },
};

const SIZE_PADDING: Record<'sm' | 'md', string> = {
  sm: '4px 10px',
  md: '8px 16px',
};

const SIZE_FONT: Record<'sm' | 'md', number> = {
  sm: 12,
  md: 13,
};

export function buttonStyle(opts: ButtonStyleOpts): React.CSSProperties {
  const v = VARIANT_COLORS[opts.variant];
  const size = opts.size ?? 'md';
  return {
    padding: SIZE_PADDING[size],
    fontSize: SIZE_FONT[size],
    background: v.bg,
    color: v.fg,
    border: '1px solid',
    borderColor: v.border,
    borderRadius: 6,
    cursor: 'pointer',
    width: opts.block ? '100%' : undefined,
  };
}