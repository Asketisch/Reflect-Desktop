/**
 * Files —— 真实文件浏览器 (B9-01).
 *
 * 左:FileTree,基于 `reflect_list_dir` 拉取的目录条目。
 * 右:CodeEditor,基于 `reflect_read_file` 拉取的文件内容 + 行号 + prism 高亮。
 * 顶:workspace 路径 + 刷新按钮。
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FolderOpen, RefreshCcw, ChevronRight } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Icon, Spinner, EmptyState } from '@/features/design-system';
import { reflect_agent_status, reflect_list_dir, reflect_read_file } from '@/utils/commands';
import { FileTree } from './FileTree';
import { CodeEditor } from './CodeEditor';
import s from './FilesView.module.css';

export function FilesView() {
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
  const fileQ = useQuery({
    queryKey: ['files-read', selected],
    queryFn: () => reflect_read_file(selected!),
    enabled: !!selected,
    staleTime: 60_000,
  });

  return (
    <PageShell
      icon={FolderOpen}
      title="Files"
      subtitle="Browse the workspace. Click any file to preview with line numbers + syntax highlighting."
      width="lg"
    >
      <div className={s.pathBar}>
        <Icon icon={FolderOpen} size={14} />
        <code className={s.workspacePath} data-testid="files-workspace-path">
          {workspace ?? '(loading…)'}
        </code>
        <button
          type="button"
          className={s.refresh}
          onClick={() => treeQ.refetch()}
          aria-label="Refresh file tree"
          data-testid="files-refresh"
        >
          <Icon icon={RefreshCcw} size={12} /> Refresh
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
            <div className={s.loading}><em>empty</em></div>
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
              title="No file selected"
              description="Pick a file from the tree on the left to preview its contents."
            />
          ) : fileQ.isLoading ? (
            <div className={s.loading}><Spinner size={18} /> <span>Loading {basename(selected)}…</span></div>
          ) : fileQ.error ? (
            <div className={s.error}>
              <Icon icon={ChevronRight} size={14} />{' '}
              <span>{(fileQ.error as Error).message}</span>
            </div>
          ) : fileQ.data ? (
            <CodeEditor
              path={fileQ.data.path}
              content={fileQ.data.content}
              binary={fileQ.data.binary}
              truncated={fileQ.data.truncated}
            />
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