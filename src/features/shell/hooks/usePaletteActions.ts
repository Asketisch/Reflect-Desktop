/**
 * usePaletteActions —— AppShell 的命令面板动作编排。
 *
 * 把原来 AppShell 里 5 个 callback(`newSession` / `clearAllSessions` /
 * `exportActive` / `saveConfig` / `runSlash`)集中,只对 palette 入口使用。
 *
 * 行为契约(从 AppShell 抽出,不可变):
 *   - `newSession` 调 `handleNewChat`(创建路由到 /chat) + pushToast。
 *   - `clearAllSessions` 通过 qc 复用 sessions 缓存,无会话直接提示,否则逐个删除并
 *     invalidate,最后统计 toast。
 *   - `exportActive` 若无 active id 提示 warn;否则调 `reflect_export_session` 并 toast。
 *   - `saveConfig` 调 `reflect_save_config('')` 并 toast;失败提示。
 *   - `runSlash` 直接 fire-and-forget 调 `submit(slash)`,让 /compact 等命令无需输入。
 *
 * 入参 `t` / `tp` 由调用方传入(可在 I18nProvider 之外的测试场景中替换)。
 */
import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAgentStore } from '@/stores/agentStore';
import {
  reflect_export_session,
  reflect_save_config,
  reflect_list_sessions,
  reflect_delete_session,
} from '@/utils/commands';

export interface PaletteToasts {
  /** "新会话已启动。" */
  newSession: string;
  /** "没有可清除的会话。" */
  noSessionsToClear: string;
  /** "删除失败：{msg}" */
  deleteFailed: (msg: string) => string;
  /** "已清除 N 个会话。"（支持复数） */
  clearedSessions: (count: number) => string;
  /** "没有可导出的活动会话。" */
  noActiveToExport: string;
  /** "已导出 → {path}" */
  exported: (path: string) => string;
  /** "已导出 →（无路径）" */
  exportedNoPath: string;
  /** "导出失败：{msg}" */
  exportFailed: (msg: string) => string;
  /** "配置已重新加载。" */
  configReloaded: string;
  /** "保存失败：{msg}" */
  saveFailed: (msg: string) => string;
}

export interface UsePaletteActionsOptions {
  activeId: string | null | undefined;
  onNewChat: () => void;
  toasts: PaletteToasts;
}

export interface UsePaletteActionsResult {
  newSession: () => void;
  clearAllSessions: () => Promise<void>;
  exportActive: () => Promise<void>;
  saveConfig: () => Promise<void>;
  runSlash: (slash: string) => void;
}

export function usePaletteActions(opts: UsePaletteActionsOptions): UsePaletteActionsResult {
  const qc = useQueryClient();
  const submit = useAgentStore((st) => st.submit);
  const pushToast = useAgentStore((st) => st.pushToast);
  const { activeId, onNewChat, toasts } = opts;

  const newSession = useCallback(() => {
    onNewChat();
    pushToast({ kind: 'info', message: toasts.newSession });
  }, [onNewChat, pushToast, toasts.newSession]);

  const clearAllSessions = useCallback(async () => {
    const list = await qc.fetchQuery({
      queryKey: ['sessions'],
      queryFn: () => reflect_list_sessions(),
    });
    if (!Array.isArray(list) || list.length === 0) {
      pushToast({ kind: 'info', message: toasts.noSessionsToClear });
      return;
    }
    for (const s of list) {
      try {
        await reflect_delete_session(s.session_id);
      } catch (e) {
        pushToast({ kind: 'error', message: toasts.deleteFailed((e as Error).message) });
      }
    }
    await qc.invalidateQueries({ queryKey: ['sessions'] });
    pushToast({ kind: 'success', message: toasts.clearedSessions(list.length) });
  }, [qc, pushToast, toasts]);

  const exportActive = useCallback(async () => {
    if (!activeId) {
      pushToast({ kind: 'warn', message: toasts.noActiveToExport });
      return;
    }
    try {
      const path = await reflect_export_session(activeId);
      pushToast({
        kind: 'success',
        message: path ? toasts.exported(path) : toasts.exportedNoPath,
      });
    } catch (e) {
      pushToast({ kind: 'error', message: toasts.exportFailed((e as Error).message) });
    }
  }, [activeId, pushToast, toasts]);

  const saveConfig = useCallback(async () => {
    try {
      await reflect_save_config('');
      pushToast({ kind: 'success', message: toasts.configReloaded });
    } catch (e) {
      pushToast({ kind: 'error', message: toasts.saveFailed((e as Error).message) });
    }
  }, [pushToast, toasts]);

  const runSlash = useCallback(
    (slash: string) => {
      // palette 触发 fire-and-forget 式 submit；这使得 /compact 等命令
      // 无需用户输入 composer 即可工作。
      void submit(slash);
    },
    [submit],
  );

  return { newSession, clearAllSessions, exportActive, saveConfig, runSlash };
}