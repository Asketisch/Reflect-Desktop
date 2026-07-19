/**
 * modals —— 阶段 3c:接通真实 reflect 事件。
 *
 * 4 个 modal 从 useAgentStore 读 pending 队列,渲染真表单,提交调 store action。
 * ModalStack 聚合所有 modal,挂到 AppLayout(同一时刻可能多个 pending)。
 *
 * - ApprovalModal    ← pendingApprovals(tool/hook/plan 审批)
 * - QuestionModal    ← pendingQuestions(AskUserQuestion 结构化选项)
 * - AskUserModal     ← pendingAskUser(AskUserInput 自由文本)
 * - PlanReadyModal   ← pendingPlan(PlanReady 计划审批)
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

/** 聚合所有 modal —— 挂到 AppLayout,根据 store 的 pending 队列决定渲染哪些。 */
export function ModalStack() {
  const approvals = useAgentStore((s) => s.pendingApprovals);
  const questions = useAgentStore((s) => s.pendingQuestions);
  const askUsers = useAgentStore((s) => s.pendingAskUser);
  const plan = useAgentStore((s) => s.pendingPlan);

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

// ====== Tool / Hook / Plan 审批 ======

function ApprovalModal({ approval }: { approval: PendingApproval }) {
  const approve = useAgentStore((s) => s.approve);
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
        <p style={{ fontFamily: 'ui-monospace, monospace', background: '#f1f5f9', padding: 8, borderRadius: 6 }}>
          {approval.toolName}
        </p>
      )}
      {approval.argsSummary && (
        <pre
          style={{
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            background: '#f8fafc',
            padding: 8,
            borderRadius: 6,
            fontSize: 12,
            maxHeight: 240,
            overflow: 'auto',
          }}
        >
          {approval.argsSummary}
        </pre>
      )}
      <p style={{ color: '#666', fontSize: 12 }}>
        Approve 运行此操作;Deny 拒绝;Approve for session 本会话内不再询问同类。
      </p>
    </ModalShell>
  );
}

// ====== AskUserQuestion(结构化选项) ======

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
  const answerQuestion = useAgentStore((s) => s.answerQuestion);
  const payload = question.payload as { questions?: QuestionItem[] };
  const items = payload?.questions ?? [];
  // 每个 question 的选中状态(index 数组)。
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
    // 构造 AskUserAnswer —— 简化:answers = 每个问题的选中 index 数组。
    // 后端 Op::AskUserQuestionResponse 接受 AskUserAnswer 结构,这里传
    // { answers: [{ indices: [...] }] } 形态(后端 serde 容错)。
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
          <div key={qIdx} style={{ marginBottom: 12 }}>
            <p style={{ fontWeight: 600, margin: '4px 0' }}>{item.question ?? item.header ?? `Question ${qIdx + 1}`}</p>
            {(item.options ?? []).map((opt, optIdx) => {
              const checked = (selections[qIdx] ?? []).includes(optIdx);
              return (
                <label key={optIdx} style={{ display: 'block', padding: '4px 0', cursor: 'pointer' }}>
                  <input
                    type={multi ? 'checkbox' : 'radio'}
                    name={`q-${question.id}-${qIdx}`}
                    checked={checked}
                    onChange={() => toggle(qIdx, optIdx, multi)}
                    style={{ marginRight: 8 }}
                  />
                  <span>{opt.label ?? `Option ${optIdx + 1}`}</span>
                  {opt.description && (
                    <span style={{ color: '#888', fontSize: 12, marginLeft: 8 }}>— {opt.description}</span>
                  )}
                </label>
              );
            })}
          </div>
        );
      })}
    </ModalShell>
  );
}

// ====== AskUserInput(自由文本) ======

function AskUserModal({ askUser }: { askUser: PendingAskUser }) {
  const answerInput = useAgentStore((s) => s.answerInput);
  const payload = askUser.payload as { prompt?: string; placeholder?: string };
  const [text, setText] = useState('');
  return (
    <ModalShell
      title="Input requested"
      open={true}
      onClose={() => answerInput(askUser.id, '')}
      primaryAction={{ label: 'Submit', onClick: () => answerInput(askUser.id, text), autoFocus: false }}
    >
      <p style={{ marginTop: 0 }}>{payload?.prompt ?? 'The agent needs your input:'}</p>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={payload?.placeholder ?? 'Type here...'}
        style={{ width: '100%', minHeight: 80, padding: 8, borderRadius: 6, border: '1px solid #cbd5e1', fontFamily: 'inherit' }}
        autoFocus
      />
    </ModalShell>
  );
}

// ====== PlanReady(计划审批) ======

function PlanReadyModal({ plan }: { plan: PendingPlan }) {
  const approve = useAgentStore((s) => s.approve);
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
      <pre
        style={{
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          background: '#f8fafc',
          padding: 12,
          borderRadius: 6,
          fontSize: 12,
          maxHeight: 320,
          overflow: 'auto',
        }}
      >
        {planText}
      </pre>
      <p style={{ color: '#666', fontSize: 12 }}>
        Approve 让 agent 按计划执行;Reject 取消并退出 plan 模式。
      </p>
    </ModalShell>
  );
}

// ====== 向后兼容:旧 stub 名(若有遗留 import) ======

export { ApprovalModal as ApprovalModalStub };
export { QuestionModal as QuestionModalStub };
export { AskUserModal as AskUserModalStub };
export { PlanReadyModal as PlanReadyModalStub };
