import type { StoreApi } from 'zustand';
import type { AgentState, Toast, ToastKind } from './types';

export const uuid = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

type SetAgentState = StoreApi<AgentState>['setState'];

export function createToastActions(set: SetAgentState) {
  return {
    pushToast: (input: { kind: ToastKind; message: string; ttlMs?: number }) => {
      const id = uuid();
      const toast: Toast = {
        id,
        kind: input.kind,
        message: input.message,
        ttlMs: input.ttlMs ?? 4000,
        createdAt: Date.now(),
      };
      set((state) => ({ toasts: [...state.toasts, toast] }));
      if (toast.ttlMs > 0 && typeof setTimeout !== 'undefined') {
        setTimeout(() => {
          set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) }));
        }, toast.ttlMs);
      }
      return id;
    },
    dismissToast: (id: string) =>
      set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) })),
  };
}
