/**
 * ToolCells —— per-tool-name rendering for tool_call TurnItems (B7-05).
 *
 * 每个 tool name(shell / file_read / file_write / web_fetch / search …)
 * 都有专属子组件:icon + args 摘要 + 可折叠 raw args。
 */
import { useState } from 'react';
import {
  Terminal,
  FileText,
  FileEdit,
  FilePlus,
  Globe,
  Search,
  Wrench,
  Loader2,
  CheckCircle2,
  XCircle,
  ChevronDown,
  ChevronRight,
  type LucideIcon,
} from 'lucide-react';
import { Icon } from '@/features/design-system';
import s from './ToolCells.module.css';

export interface ToolCellProps {
  toolName: string;
  argsSummary: string;
  status: 'running' | 'done' | 'error';
}

const ICON_MAP: Record<string, LucideIcon> = {
  shell: Terminal,
  bash: Terminal,
  read_file: FileText,
  file_read: FileText,
  write_file: FileEdit,
  file_write: FileEdit,
  create_file: FilePlus,
  web_fetch: Globe,
  web_search: Search,
  search: Search,
};

function pickIcon(name: string): LucideIcon {
  return ICON_MAP[name] ?? Wrench;
}

function parseArgs(args: string): Record<string, unknown> | null {
  try {
    return JSON.parse(args);
  } catch {
    return null;
  }
}

function StatusIcon({ status }: { status: ToolCellProps['status'] }) {
  if (status === 'running') return <Icon icon={Loader2} size={13} className={s.spin} />;
  if (status === 'done') return <Icon icon={CheckCircle2} size={13} />;
  return <Icon icon={XCircle} size={13} />;
}

function summarize(name: string, args: Record<string, unknown> | null): string {
  if (!args) return name;
  switch (name) {
    case 'shell':
    case 'bash':
      return `$ ${args.command ?? args.cmd ?? ''}`;
    case 'read_file':
    case 'file_read':
      return `${args.path ?? args.file ?? '(no path)'}`;
    case 'write_file':
    case 'file_write':
      return `${args.path ?? args.file ?? ''} (${args.content ? `${String(args.content).length} chars` : 'edit'})`;
    case 'create_file':
      return `+ ${args.path ?? args.file ?? ''}`;
    case 'web_fetch':
    case 'web_search':
      return `${args.url ?? args.query ?? ''}`;
    default:
      return name;
  }
}

export function ToolCell({ toolName, argsSummary, status }: ToolCellProps) {
  const Icon2 = pickIcon(toolName);
  const args = parseArgs(argsSummary);
  const summary = summarize(toolName, args);
  const [open, setOpen] = useState(false);
  const accent = status === 'error' ? 'danger' : status === 'done' ? 'success' : 'warning';

  return (
    <div
      className={s.cell}
      data-tool={toolName}
      data-status={status}
      data-testid={`tool-cell-${toolName}`}
    >
      <button
        type="button"
        className={s.header}
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <Icon icon={open ? ChevronDown : ChevronRight} size={12} className={s.caret} />
        <Icon icon={Icon2} size={13} className={s[`accent_${accent}`]} />
        <span className={s.name}>{toolName}</span>
        <span className={s.summary}>{summary}</span>
        <StatusIcon status={status} />
      </button>
      {open && (
        <pre className={s.args} data-testid={`tool-args-${toolName}`}>
          {argsSummary || '(no args)'}
        </pre>
      )}
    </div>
  );
}
