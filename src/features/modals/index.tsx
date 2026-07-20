/**
 * modals —— 接通真实 reflect 事件的 4 个 modal（CSS Modules 版）。
 *
 * - ApprovalModal    ← pendingApprovals
 * - QuestionModal    ← pendingQuestions
 * - AskUserModal     ← pendingAskUser
 * - PlanReadyModal   ← pendingPlan
 */
import { useState } from 'react';
import { ModalShell } from './ModalShell';
import {
  useAgentStore,
  type PendingApproval,
  type PendingQuestion,
  type PendingAskUser,
  type PendingPlan,
} from '@/stores/agentStore';
import s from './index.module.css';

/** 聚合所有 modal。 */
export function ModalStack() {
  const approvals = useAgentStore((st) => st.pendingApprovals);
  const questions = useAgentStore((st) => st.pendingQuestions);
  const askUsers = useAgentStore((st) => st.pendingAskUser);
  const plan = useAgentStore((st) => st.pendingPlan);

  return (
    <>
      {approvals.map((a) => (
        <ApprovalModal key={`ap-${a.id}`} approval={a} />
      ))}
      {questions.map((q) => (
        <QuestionModal key={`q-${q.id}`} question={q} />
      ))}
      {askUsers.map((u) => (
        <AskUserModal key={`u-${u.id}`} askUser={u} />
      ))}
      {plan && <PlanReadyModal plan={plan} />}
    </>
  );
}

function ApprovalModal({ approval }: { approval: PendingApproval }) {
  const approve = useAgentStore((st) => st.approve);
  const kindLabel = approval.kind === 'tool' ? 'Tool Approval' : approval.kind === 'hook' ? 'Hook Approval' : 'Plan Approval';
  return (
    <ModalShell
      title={kindLabel}
      open={true}
      onClose={() => approve(approval.kind, approval.id, { deny: { reason: 'dismissed' } })}
      tertiaryAction={{
        label: 'Deny',
        onClick: () => approve(approval.kind, approval.id, { deny: { reason: 'denied by user' } }),
      }}
      secondaryAction={{
        label: 'Approve for session',
        onClick: () => approve(approval.kind, approval.id, 'approve_for_session'),
      }}
      primaryAction={{
        label: 'Approve',
        onClick: () => approve(approval.kind, approval.id, 'approve'),
        autoFocus: true,
      }}
    >
      {approval.toolName && (
        <div className={s.codeBlock}>{approval.toolName}</div>
      )}
      {approval.argsSummary && (
        <pre className={s.argsBlock}>{approval.argsSummary}</pre>
      )}
      <p className={s.hint}>
        Approve runs this action once. Approve for session skips future prompts of the same kind.
      </p>
    </ModalShell>
  );
}

interface QuestionOption {
  label?: string;
  description?: string;
}
interface QuestionItem {
  question?: string;
  header?: string;
  options?: QuestionOption[];
  multiSelect?: boolean;
}

function QuestionModal({ question }: { question: PendingQuestion }) {
  const answerQuestion = useAgentStore((st) => st.answerQuestion);
  const payload = question.payload as { questions?: QuestionItem[] };
  const items = payload?.questions ?? [];
  const [selections, setSelections] = useState<Record<number, number[]>>({});

  const toggle = (qIdx: number, optIdx: number, multi: boolean) => {
    setSelections((prev) => {
      const cur = prev[qIdx] ?? [];
      if (multi) {
        return { ...prev, [qIdx]: cur.includes(optIdx) ? cur.filter((i) => i !== optIdx) : [...cur, optIdx] };
      }
      return { ...prev, [qIdx]: [optIdx] };
    });
  };

  const submit = () => {
    const answers = items.map((_, qIdx) => ({
      indices: selections[qIdx] ?? [],
    }));
    answerQuestion(question.id, { answers });
  };

  return (
    <ModalShell
      title="Question"
      open={true}
      onClose={() => answerQuestion(question.id, { answers: [] })}
      primaryAction={{ label: 'Submit', onClick: submit, autoFocus: false }}
    >
      {items.length === 0 && <p>(empty question payload)</p>}
      {items.map((item, qIdx) => {
        const multi = Boolean(item.multiSelect);
        return (
          <div key={qIdx} className={s.questionItem}>
            <p className={s.questionText}>{item.question ?? item.header ?? `Question ${qIdx + 1}`}</p>
            <div className={s.optionList}>
              {(item.options ?? []).map((opt, optIdx) => {
                const checked = (selections[qIdx] ?? []).includes(optIdx);
                return (
                  <label key={optIdx} className={s.option}>
                    <input
                      type={multi ? 'checkbox' : 'radio'}
                      name={`q-${question.id}-${qIdx}`}
                      checked={checked}
                      onChange={() => toggle(qIdx, optIdx, multi)}
                      className={s.optionInput}
                    />
                    <span className={s.optionLabel}>{opt.label ?? `Option ${optIdx + 1}`}</span>
                    {opt.description && (
                      <span className={s.optionDesc}>— {opt.description}</span>
                    )}
                  </label>
                );
              })}
            </div>
          </div>
        );
      })}
    </ModalShell>
  );
}

function AskUserModal({ askUser }: { askUser: PendingAskUser }) {
  const answerInput = useAgentStore((st) => st.answerInput);
  const payload = askUser.payload as { prompt?: string; placeholder?: string };
  const [text, setText] = useState('');
  return (
    <ModalShell
      title="Input requested"
      open={true}
      onClose={() => answerInput(askUser.id, '')}
      primaryAction={{ label: 'Submit', onClick: () => answerInput(askUser.id, text), autoFocus: false }}
    >
      <p className={s.promptText}>{payload?.prompt ?? 'The agent needs your input:'}</p>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={payload?.placeholder ?? 'Type here...'}
        className={s.textarea}
        autoFocus
      />
    </ModalShell>
  );
}

function PlanReadyModal({ plan }: { plan: PendingPlan }) {
  const approve = useAgentStore((st) => st.approve);
  const payload = plan.payload as { plan?: string; summary?: string; steps?: unknown[] };
  const planText = payload?.summary ?? payload?.plan ?? JSON.stringify(plan.payload, null, 2);
  return (
    <ModalShell
      title="Plan Ready"
      open={true}
      onClose={() => approve('plan', plan.id, { deny: { reason: 'plan dismissed' } })}
      tertiaryAction={{
        label: 'Reject',
        onClick: () => approve('plan', plan.id, { deny: { reason: 'plan rejected' } }),
      }}
      primaryAction={{
        label: 'Approve plan',
        onClick: () => approve('plan', plan.id, 'approve'),
        autoFocus: true,
      }}
    >
      <pre className={s.planBlock}>{planText}</pre>
      <p className={s.hint}>
        Approve lets the agent execute the plan; Reject cancels and exits plan mode.
      </p>
    </ModalShell>
  );
}

// ====== 向后兼容：旧 stub 名 ======
export { ApprovalModal as ApprovalModalStub };
export { QuestionModal as QuestionModalStub };
export { AskUserModal as AskUserModalStub };
export { PlanReadyModal as PlanReadyModalStub };
