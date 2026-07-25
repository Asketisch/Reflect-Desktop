/**
 * i18n namespace —— inspector.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const inspector: Record<string, StringEntry> = {
  'inspector.title':             { en: 'Inspector',                   'zh-CN': '检查器' },
  'inspector.pending':           { en: 'Pending',                     'zh-CN': '待处理' },
  'inspector.pendingEmpty':      { en: 'No pending interactions.',    'zh-CN': '没有待处理交互。' },
  'inspector.pendingApprovals':  { en: 'Approvals',                   'zh-CN': '权限' },
  'inspector.pendingQuestions':  { en: 'Questions',                   'zh-CN': '问题' },
  'inspector.pendingInputs':     { en: 'Inputs',                      'zh-CN': '输入' },
  'inspector.mcpServers':        { en: 'MCP servers',                 'zh-CN': 'MCP 服务' },
  'inspector.mcpEmpty':          { en: 'No MCP servers configured.',  'zh-CN': '未配置 MCP 服务。' },
  'inspector.lspServers':        { en: 'LSP servers',                 'zh-CN': 'LSP 服务' },
  'inspector.lspEmpty':          { en: 'No LSP servers configured.',  'zh-CN': '未配置 LSP 服务。' },
  'inspector.lastError':         { en: 'Last error',                  'zh-CN': '最近错误' },
  'inspector.dismiss':           { en: 'Dismiss',                     'zh-CN': '忽略' },
};

export default inspector;
