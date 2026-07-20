/**
 * Files —— 通过 agent 探索文件（CSS Modules 版）。
 * 快捷按钮发预设 prompt 给 agent。
 */
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FolderOpen, FileSearch, FileClock, ArrowRight, Check } from 'lucide-react';
import type { ComponentType } from 'react';
import { useAgentStore } from '@/stores/agentStore';
import { reflect_agent_status } from '@/utils/tauri';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Icon } from '@/features/design-system';
import s from './FilesView.module.css';

interface QuickPrompt {
  label: string;
  prompt: string;
  icon: ComponentType;
}

const QUICK_PROMPTS: QuickPrompt[] = [
  {
    label: 'List root files',
    prompt: '请用 glob 工具列出当前工作区根目录的文件和子目录。',
    icon: FolderOpen,
  },
  {
    label: 'Find recently modified',
    prompt: '请找出工作区内最近修改过的 10 个文件(用 bash: git log 或 find -mtime)。',
    icon: FileClock,
  },
  {
    label: 'Search by name',
    prompt: '我想按文件名搜索,请告诉我你用 glob 工具需要什么 pattern。',
    icon: FileSearch,
  },
];

export function FilesView() {
  const submit = useAgentStore((st) => st.submit);
  const [sent, setSent] = useState<string | null>(null);
  const statusQ = useQuery({
    queryKey: ['agent-status'],
    queryFn: reflect_agent_status,
    staleTime: 30_000,
  });

  const send = async (prompt: string) => {
    await submit(prompt);
    setSent(prompt);
    setTimeout(() => setSent(null), 2000);
  };

  return (
    <PageShell
      icon={FolderOpen}
      title="Files"
      subtitle="Explore the workspace via the agent's read / glob / grep tools."
      width="md"
    >
      <Card level="outlined" padding="md" className={s.workspaceCard}>
        <div className={s.workspaceRow}>
          <Icon icon={FolderOpen} size={14} />
          <span className={s.workspaceLabel}>Workspace</span>
          <code className={s.workspacePath}>{statusQ.data?.workspace ?? '(loading…)'}</code>
        </div>
        <p className={s.note}>
          File browsing is handled by the agent's <code className={s.codeInline}>read</code> /{' '}
          <code className={s.codeInline}>glob</code> / <code className={s.codeInline}>grep</code> tools.
          Send a quick action below or describe what you need directly in Chat.
        </p>
      </Card>

      <h3 className={s.sectionTitle}>Quick actions</h3>
      <div className={s.grid}>
        {QUICK_PROMPTS.map((q) => (
          <button
            key={q.label}
            onClick={() => send(q.prompt)}
            className={s.actionCard}
            data-sent={sent === q.prompt || undefined}
          >
            <div className={s.actionIcon}>
              <Icon icon={sent === q.prompt ? Check : q.icon} size={18} />
            </div>
            <div className={s.actionBody}>
              <div className={s.actionLabel}>{q.label}</div>
            </div>
            <Icon icon={ArrowRight} size={14} className={s.actionArrow} />
          </button>
        ))}
      </div>
      <p className={s.hint}>After sending, switch to Chat to see results.</p>
    </PageShell>
  );
}
