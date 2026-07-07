/**
 * M1.6 简化 4 种 modal —— MinimalAgent 后端不 emit Approval/PendingQuestion/
 * PendingAskUser/PlanRequest,这些 modal 仅占位 stub。M2.x 接入真实事件后启用。
 */
import { ModalShell } from './ModalShell';

export function ApprovalModalStub() {
  return (
    <ModalShell
      title="Tool Approval"
      open={false}
      onClose={() => {}}
    >
      <p style={{ color: '#888', fontSize: 12 }}>
        占位 —— M2.x 接入真实 ApprovalRequest 事件后启用 (见 docs/todo/21-gui/06-modals.md)。
      </p>
    </ModalShell>
  );
}

export function QuestionModalStub() {
  return (
    <ModalShell title="Question" open={false} onClose={() => {}}>
      <p>占位 —— M2.x 启用。</p>
    </ModalShell>
  );
}

export function AskUserModalStub() {
  return (
    <ModalShell title="Ask User" open={false} onClose={() => {}}>
      <p>占位 —— M2.x 启用。</p>
    </ModalShell>
  );
}

export function PlanReadyModalStub() {
  return (
    <ModalShell title="Plan Ready" open={false} onClose={() => {}}>
      <p>占位 —— M2.x 启用。</p>
    </ModalShell>
  );
}
