/**
 * Tooltip —— 纯 CSS tooltip（hover 显示）。
 *
 * 用法：
 *   <Tooltip label="Settings">
 *     <IconButton ...><GearIcon/></IconButton>
 *   </Tooltip>
 *
 * 位置默认 top，可通过 side 切换。延迟由 CSS transition 控制。
 */
import type { ReactNode } from 'react';
import s from './Tooltip.module.css';

export interface TooltipProps {
  label: ReactNode;
  side?: 'top' | 'bottom' | 'left' | 'right';
  children: ReactNode;
}

export function Tooltip({ label, side = 'top', children }: TooltipProps) {
  return (
    <span className={s.wrap} data-side={side}>
      {children}
      <span className={s.tip} role="tooltip">
        {label}
      </span>
    </span>
  );
}
