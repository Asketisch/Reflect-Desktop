/**
 * design-system slice public API barrel.
 */

// ===== Views =====
export { DesignSystemView } from './DesignSystemView';

// ===== Primitives =====
export { Button } from './primitives/Button';
export type { ButtonProps, ButtonVariant, ButtonSize } from './primitives/Button';

export { IconButton } from './primitives/IconButton';
export type { IconButtonProps } from './primitives/IconButton';

export { Icon } from './primitives/Icon';
export type { IconProps } from './primitives/Icon';

export { Input } from './primitives/Input';
export type { InputProps } from './primitives/Input';

export { Textarea } from './primitives/Textarea';
export type { TextareaProps } from './primitives/Textarea';

export { Select } from './primitives/Select';
export type { SelectProps } from './primitives/Select';

export { SegmentedControl } from './primitives/SegmentedControl';
export type { SegmentedControlProps, SegmentedOption } from './primitives/SegmentedControl';

export { Badge } from './primitives/Badge';
export type { BadgeProps, BadgeVariant } from './primitives/Badge';

export { Card } from './primitives/Card';
export type { CardProps } from './primitives/Card';

export { EmptyState } from './primitives/EmptyState';
export type { EmptyStateProps } from './primitives/EmptyState';

export { Spinner } from './primitives/Spinner';
export type { SpinnerProps } from './primitives/Spinner';

export { Tooltip } from './primitives/Tooltip';
export type { TooltipProps } from './primitives/Tooltip';

export { Toast } from './primitives/Toast';
export type { ToastProps } from './primitives/Toast';
export type { ToastKind } from './utils/toast';

export { ContextRing } from './primitives/ContextRing';
export type { ContextRingProps } from './primitives/ContextRing';
export type { RingSegment } from './utils/ring';

export { KeyHint } from './primitives/KeyHint';
export type { KeyHintProps } from './primitives/KeyHint';

// ===== Utils (保留 token 常量供外部用) =====
export { shortcutLabel, platformLabel, detectPlatform } from './utils/keyHints';
export type { Platform } from './utils/keyHints';

export { ringGeometry, segmentArc } from './utils/ring';
export type { RingGeometry } from './utils/ring';

export { TOAST_COLORS, DEFAULT_TOAST_MS } from './utils/toast';
