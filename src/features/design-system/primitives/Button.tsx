/**
 * Button —— 全局 button primitive。
 *
 * CodexMonitor 同名: `src/components/Button.tsx`
 */
import { forwardRef } from 'react';
import type { ButtonHTMLAttributes } from 'react';
import { buttonStyle, type ButtonVariant } from '../utils/buttonStyles';

export interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'type'> {
  variant?: ButtonVariant;
  size?: 'sm' | 'md';
  block?: boolean;
  type?: 'button' | 'submit' | 'reset';
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = 'secondary', size = 'md', block, type = 'button', style, children, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      style={{ ...buttonStyle({ variant, size, block }), ...style }}
      {...rest}
    >
      {children}
    </button>
  );
});