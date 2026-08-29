/**
 * Files —— 真实文件浏览器 (B9-01)。
 *
 * 左:FileTree,基于 `reflect_list_dir` 拉取的目录条目。
 * 右:CodeMirrorEditor(CM6,可编辑 + 语法高亮);agent 编辑过的文件叠加
 * Cursor 式变更装饰(绿/红 + 逐块 Accept/Reject),Reject 经
 * `editorStore.rejectHunk` inverse-patch 恢复原文并写回磁盘。
 * 顶:workspace 路径 + 刷新按钮。
 */
import { useEffect, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { FolderOpen, RefreshCcw, ChevronRight, Check, Undo2, Zap } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Icon, Spinner, EmptyState, Badge, Button } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import {
  reflect_agent_status,
  reflect_list_dir,
  reflect_read_file,
  reflect_lsp_set_enabled,
  reflect_lsp_status,
  reflect_lsp_warmup,
  type LspStatus,
  type ReflectFileReadResult,
} from '@/utils/commands';

/** 按工作区持久化 LSP 开关（默认关闭——多项目同时起语言服务会拖累系统）。 */
export function lspPrefKey(workspace: string | null): string {
  return `reflect.lsp.enabled.${workspace ?? 'none'}`;
}

export function readLspPref(workspace: string | null): boolean {
  try {
    return localStorage.getItem(lspPrefKey(workspace)) === 'on';
  } catch {
    return false;
  }
}

function writeLspPref(workspace: string | null, enabled: boolean): void {
  try {
    localStorage.setItem(lspPrefKey(workspace), enabled ? 'on' : 'off');
  } catch {
    // localStorage 不可用（隐私模式等）—— 仅本会话生效。
  }
}
import { useEditorStore } from '@/stores/editorStore';
import { FileTree } from './FileTree';
import { CodeMirrorEditor } from './CodeMirrorEditor';
import s from './FilesView.module.css';

export function FilesView() {
  const { t } = useI18n();
  const qc = useQueryClient();
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });
  const workspace = statusQ.data?.workspace ?? null;
  const treeQ = useQuery({
    queryKey: ['files-tree', workspace],
    queryFn: () => reflect_list_dir(workspace, 4),
    enabled: !!workspace,
    staleTime: 10_000,
  });
  const [selected, setSelected] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const fileQ = useQuery({
    queryKey: ['files-read', selected],
    queryFn: () => reflect_read_file(selected!),
    enabled: !!selected,
    staleTime: 60_000,
  });

  // LSP:按工作区手动开启(默认关);开启状态跨会话恢复。
  const [lsp, setLsp] = useState<LspStatus>({ enabled: false, servers: [] });
  useEffect(() => {
    // 切工作区:恢复该工作区的偏好(默认关)。
    const pref = readLspPref(workspace);
    setLsp({ enabled: false, servers: [] });
    reflect_lsp_status()
      .then((st) => {
        if (st.enabled !== pref) {
          return reflect_lsp_set_enabled(pref);
        }
        return st;
      })
      .then((st) => {
        if (st) setLsp(st);
      })
      .catch(() => {/* 状态不可读时保持关闭 */});
  }, [workspace]);

  const onToggleLsp = () => {
    const next = !lsp.enabled;
    writeLspPref(workspace, next);
    setLsp((prev) => ({ ...prev, enabled: next }));
    reflect_lsp_set_enabled(next)
      .then(setLsp)
      .catch(() => {/* 失败回退由下一次 status 查询校正 */});
  };

  // 文件预热:真正打开代码文件后才触发 didOpen(VSCode 式按需索引);
  // LSP 未开启时不调用。
  useEffect(() => {
    if (!lsp.enabled || !fileQ.data || fileQ.data.binary || !selected) return;
    void reflect_lsp_warmup(fileQ.data.path).catch(() => {/* 尽力而为 */});
  }, [lsp.enabled, fileQ.data, selected]);

  const pending = useEditorStore((st) => (selected ? st.pending[selected] : undefined));
  const lastEditedPath = useEditorStore((st) => st.lastEditedPath);
  const acceptHunk = useEditorStore((st) => st.acceptHunk);
  const acceptAll = useEditorStore((st) => st.acceptAll);
  const rejectHunk = useEditorStore((st) => st.rejectHunk);
  const rejectAll = useEditorStore((st) => st.rejectAll);

  // agent 刚编辑过某文件 → 自动聚焦到该文件(编辑器打开即可 review)。
  useEffect(() => {
    if (lastEditedPath) setSelected(lastEditedPath);
  }, [lastEditedPath]);

  /** Reject 落盘成功后把恢复内容直接写进查询缓存。
   *
   * 不能走 invalidateQueries + 等 refetch：窗口期内 `value` 仍是
   * agent 版本,CodeMirrorEditor 会把刚恢复的缓冲整体替换回旧内容
   * （Reject All 则根本没人把恢复值写回缓冲)——磁盘与编辑器显示
   * 从此不一致。内容已确定,直接 setQueryData 即时生效。 */
  const syncRecoveredContent = (next: string) => {
    if (!selected) return;
    qc.setQueryData<ReflectFileReadResult>(['files-read', selected], (old) =>
      old ? { ...old, content: next } : old,
    );
  };

  const onAcceptHunk = (index: number) => {
    if (!selected) return;
    acceptHunk(selected, index);
  };

  const onAcceptAll = () => {
    if (!selected) return;
    acceptAll(selected);
  };

  const onRejectHunk = async (index: number, currentContent: string): Promise<string | null> => {
    if (!selected) return null;
    const next = await rejectHunk(selected, index, currentContent);
    if (next === null) {
      setActionError(t('files.rejectFailed'));
      return null;
    }
    setActionError(null);
    syncRecoveredContent(next);
    return next;
  };

  const onRejectAll = async (currentContent: string): Promise<string | null> => {
    if (!selected) return null;
    const next = await rejectAll(selected, currentContent);
    if (next === null) {
      setActionError(t('files.rejectFailed'));
      return null;
    }
    setActionError(null);
    syncRecoveredContent(next);
    return next;
  };

  const path = fileQ.data?.path ?? selected ?? '';
  const hunks = pending?.hunks ?? [];

  return (
    <PageShell
      icon={FolderOpen}
      title={t('files.title')}
      subtitle={t('files.subtitle')}
      width="lg"
    >
      <div className={s.pathBar}>
        <Icon icon={FolderOpen} size={14} />
        <code className={s.workspacePath} data-testid="files-workspace-path">
          {workspace ?? t('common.loading')}
        </code>
        <button
          type="button"
          className={s.refresh}
          onClick={() => treeQ.refetch()}
          aria-label={t('files.refresh')}
          data-testid="files-refresh"
        >
          <Icon icon={RefreshCcw} size={12} /> {t('files.refresh')}
        </button>
        <button
          type="button"
          className={s.lspChip}
          data-enabled={lsp.enabled || undefined}
          onClick={onToggleLsp}
          data-testid="files-lsp-toggle"
          title={lsp.enabled ? t('files.lspOnHint') : t('files.lspOffHint')}
        >
          <Icon icon={Zap} size={12} />
          {lsp.enabled
            ? t('files.lspOn', { count: String(lsp.servers.length) })
            : t('files.lspOff')}
        </button>
      </div>

      <div className={s.split}>
        <aside className={s.treePane} data-testid="files-tree-pane">
          {treeQ.isLoading ? (
            <div className={s.loading}><Spinner size={18} /></div>
          ) : treeQ.error ? (
            <div className={s.error}>
              <Icon icon={FolderOpen} size={14} />{' '}
              <span>{(treeQ.error as Error).message}</span>
            </div>
          ) : treeQ.data && treeQ.data.entries.length === 0 ? (
            <div className={s.loading}><em>{t('files.empty')}</em></div>
          ) : (
            <FileTree
              entries={treeQ.data?.entries ?? []}
              selectedPath={selected}
              onSelectFile={setSelected}
            />
          )}
        </aside>

        <section className={s.previewPane} data-testid="files-preview-pane">
          {!selected ? (
            <EmptyState
              icon={<Icon icon={FolderOpen} />}
              title={t('files.noFile')}
              description={t('files.noFileDesc')}
            />
          ) : fileQ.isLoading ? (
            <div className={s.loading}><Spinner size={18} /> <span>{t('files.loadingFile', { name: basename(selected) })}</span></div>
          ) : fileQ.error ? (
            <div className={s.error}>
              <Icon icon={ChevronRight} size={14} />{' '}
              <span>{(fileQ.error as Error).message}</span>
            </div>
          ) : fileQ.data && fileQ.data.binary ? (
            <div className={s.loading}><em>{t('files.binary')}</em></div>
          ) : fileQ.data ? (
            <div className={s.editorWrap}>
              {hunks.length > 0 && (
                <div className={s.reviewBar} data-testid="files-review-bar">
                  <Badge variant="warning">{t('files.pendingChanges', { count: hunks.length })}</Badge>
                  <Button
                    variant="ghost"
                    size="sm"
                    data-testid="files-reject-all"
                    onClick={() => void onRejectAll(fileQ.data!.content)}
                  >
                    <Icon icon={Undo2} size={12} /> {t('files.rejectAll')}
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    data-testid="files-accept-all"
                    onClick={onAcceptAll}
                  >
                    <Icon icon={Check} size={12} /> {t('files.acceptAll')}
                  </Button>
                </div>
              )}
              {actionError && <p className={s.actionError}>{actionError}</p>}
              <CodeMirrorEditor
                path={path}
                value={fileQ.data.content}
                revision={0}
                hunks={hunks}
                onAcceptHunk={onAcceptHunk}
                onRejectHunk={onRejectHunk}
              />
            </div>
          ) : null}
        </section>
      </div>
    </PageShell>
  );
}

function basename(p: string): string {
  const i = p.lastIndexOf('/');
  if (i < 0) return p;
  return p.slice(i + 1);
}
