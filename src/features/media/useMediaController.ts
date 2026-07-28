/**
 * useMediaController —— Media Studio + Computer Use Tab 控制器 (Phase 3 item 13).
 *
 * 数据源:
 * - `reflect_list_media` —— 目录扫图(Studio tab)。
 * - `reflect_media_capabilities` —— 后端能力诊断。
 * - `reflect_computer_use` —— 动作执行(Computer Use tab)。
 */
import { useCallback, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  reflect_list_media,
  reflect_media_capabilities,
  reflect_computer_use,
  type ReflectComputerUseAction,
  type ReflectMediaAsset,
  type ReflectMediaCapabilities,
} from '@/utils/commands/media';
import { useAgentStore } from '@/stores/agentStore';

export const MEDIA_QUERY_KEY = ['media', 'assets'] as const;
const MEDIA_STALE_MS = 15_000;

export type MediaTab = 'studio' | 'computer';

export interface MediaController {
  tab: MediaTab;
  setTab: (t: MediaTab) => void;
  /** Studio state */
  studioDir: string;
  setStudioDir: (d: string) => void;
  assets: ReflectMediaAsset[];
  studioLoading: boolean;
  studioError: Error | null;
  refreshStudio: () => void;
  /** Computer state */
  actionHistory: ReflectComputerUseAction[];
  executeAction: (action: ReflectComputerUseAction) => Promise<void>;
  busy: boolean;
  lastError: string | null;
  /** Capabilities */
  capabilities: ReflectMediaCapabilities | null;
}

export function useMediaController(): MediaController {
  const qc = useQueryClient();
  const pushToast = useAgentStore((st) => st.pushToast);
  const [tab, setTab] = useState<MediaTab>('studio');
  // Default to a benign scratch path; user typically overrides this in the UI.
  // Empty string keeps the studio query disabled until a path is entered.
  const [studioDir, setStudioDir] = useState<string>('/tmp');

  const studioQ = useQuery({
    queryKey: [...MEDIA_QUERY_KEY, studioDir] as const,
    queryFn: () => reflect_list_media(studioDir),
    enabled: !!studioDir && tab === 'studio',
    staleTime: MEDIA_STALE_MS,
    retry: false,
  });

  const capsQ = useQuery({
    queryKey: ['media', 'capabilities'] as const,
    queryFn: () => reflect_media_capabilities(),
    staleTime: 60_000,
  });

  const [actionHistory, setActionHistory] = useState<ReflectComputerUseAction[]>([]);
  const [lastError, setLastError] = useState<string | null>(null);

  const actionMut = useMutation({
    mutationFn: async (action: ReflectComputerUseAction) => {
      await reflect_computer_use(action);
    },
    onSuccess: (_data, action) => {
      pushToast({ kind: 'success', message: `executed: ${summarizeAction(action)}` });
      setActionHistory((h) => [action, ...h].slice(0, 20));
      setLastError(null);
    },
    onError: (e) => {
      const msg = String(e);
      setLastError(msg);
      pushToast({ kind: 'warn', message: msg });
    },
  });

  const refreshStudio = useCallback(() => {
    void qc.invalidateQueries({ queryKey: [...MEDIA_QUERY_KEY] });
  }, [qc]);

  const executeAction = useCallback(
    async (action: ReflectComputerUseAction) => {
      await actionMut.mutateAsync(action);
    },
    [actionMut],
  );

  return {
    tab,
    setTab,
    studioDir,
    setStudioDir,
    assets: studioQ.data ?? [],
    studioLoading: studioQ.isLoading,
    studioError: studioQ.error as Error | null,
    refreshStudio,
    actionHistory,
    executeAction,
    busy: actionMut.isPending,
    lastError,
    capabilities: capsQ.data ?? null,
  };
}

function summarizeAction(a: ReflectComputerUseAction): string {
  switch (a.kind) {
    case 'screenshot':
      return 'Screenshot';
    case 'mouseMove':
      return `Mouse move (${a.params.x}, ${a.params.y})`;
    case 'mouseClick':
      return `Click (${a.params.x}, ${a.params.y})`;
    case 'keyType':
      return `Type "${a.params.text.slice(0, 20)}"`;
    case 'keyCombo':
      return `Combo ${a.params.keys}`;
    case 'scroll':
      return `Scroll (${a.params.dx}, ${a.params.dy})`;
  }
}
