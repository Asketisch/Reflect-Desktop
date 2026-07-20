/**
 * Git —— 通过 agent 操作 git（CSS Modules 版）。
 */
import { useState } from 'react';
import { GitBranch, GitCommitHorizontal, GitPullRequest, Upload, Check, ArrowRight } from 'lucide-react';
import type { ComponentType } from 'react';
import { useAgentStore } from '@/stores/agentStore';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Icon } from '@/features/design-system';
import s from './GitView.module.css';

interface QuickPrompt {
  label: string;
  prompt: string;
  icon: ComponentType;
}

const QUICK_PROMPTS: QuickPrompt[] = [
  {
    label: 'Branch & status',
    prompt: '请用 bash 运行 `git status -sb` 并总结当前分支和工作区变更。',
    icon: GitBranch,
  },
  {
    label: 'Recent commits',
    prompt: '请用 bash 运行 `git log --oneline -10` 展示最近 10 条提交。',
    icon: GitCommitHorizontal,
  },
  {
    label: 'Unpushed commits',
    prompt: '请用 bash 运行 `git log origin/HEAD..HEAD --oneline`(若失败说明无上游)。',
    icon: Upload,
  },
  {
    label: 'Create commit',
    prompt: '我想创建一个 git commit。请先用 git status 和 git diff 查看变更,然后建议 commit message。',
    icon: GitPullRequest,
  },
];

export function GitView() {
  const submit = useAgentStore((st) => st.submit);
  const [sent, setSent] = useState<string | null>(null);

  const send = async (prompt: string) => {
    await submit(prompt);
    setSent(prompt);
    setTimeout(() => setSent(null), 2000);
  };

  return (
    <PageShell
      icon={GitBranch}
      title="Git"
      subtitle="Run git operations via the agent's bash tool."
      width="md"
    >
      <Card level="outlined" padding="md" className={s.infoCard}>
        <p className={s.note}>
          Git operations are performed by the agent's <code className={s.codeInline}>bash</code> tool.
          Use a quick action below or describe what you need in Chat (e.g. "commit and push").
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
    </PageShell>
  );
}
