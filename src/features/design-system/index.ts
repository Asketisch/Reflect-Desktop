/**
 * design-system slice public API barrel.
 */
export { DesignSystemView } from './DesignSystemView';

export { Button } from './primitives/Button';
export type { ButtonProps } from './primitives/Button';

export { Toast } from './primitives/Toast';
export type { ToastProps } from './primitives/Toast';
export type { ToastKind } from './utils/toast';

export { ContextRing } from './primitives/ContextRing';
export type { ContextRingProps } from './primitives/ContextRing';
export type { RingSegment } from './utils/ring';

export { KeyHint } from './primitives/KeyHint';
export type { KeyHintProps } from './primitives/KeyHint';

export { buttonStyle } from './utils/buttonStyles';
export type { ButtonVariant, ButtonStyleOpts } from './utils/buttonStyles';

export { ringGeometry, segmentArc } from './utils/ring';
export type { RingGeometry } from './utils/ring';

export { shortcutLabel, platformLabel, detectPlatform } from './utils/keyHints';
export type { Platform } from './utils/keyHints';

export { TOAST_COLORS, DEFAULT_TOAST_MS } from './utils/toast';