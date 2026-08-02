/**
 * i18n namespace —— collaboration.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const collaboration: Record<string, StringEntry> = {
  'collaboration.title':       { en: 'Collaboration & Extensions',         'zh-CN': '协作与扩展' },
  'collaboration.subtitle':    { en: 'External tools and language servers connected to the agent.', 'zh-CN': '连接到智能体的外部工具和语言服务器。' },
  'collaboration.empty':       { en: 'No collaborators yet.',              'zh-CN': '暂无协作者。' },
  'collaboration.invite':      { en: 'Invite',                             'zh-CN': '邀请' },
  'collaboration.share':       { en: 'Share',                              'zh-CN': '分享' },
  'collaboration.mcpTitle':    { en: 'MCP Servers',                        'zh-CN': 'MCP 服务器' },
  'collaboration.mcpEmpty':    { en: 'No MCP servers configured',          'zh-CN': '未配置 MCP 服务器' },
  'collaboration.mcpHint':     { en: '{count} MCP tools registered (prefix <code className={s.codeInline}>mcp__</code>).', 'zh-CN': '{count} 个 MCP 工具已注册（前缀 mcp__）。' },
  'collaboration.lspTitle':    { en: 'LSP Servers',                        'zh-CN': 'LSP 服务器' },
  'collaboration.lspEmpty':    { en: 'No LSP servers configured',          'zh-CN': '未配置 LSP 服务器' },
  'collaboration.futureTitle': { en: 'Multi-agent orchestration',        'zh-CN': '多智能体编排' },
  'collaboration.futureText':  { en: 'Subagent / discussion / task / pipeline / goal orchestration is a core reflect-agent capability. GUI triggers will arrive in a later phase — for now, ask the agent in Chat (e.g. "use a subagent to parallelize").', 'zh-CN': '子智能体/讨论/任务/流水线/目标编排是 reflect-agent 的核心能力。GUI 触发器将在后续阶段添加——目前，请在聊天中询问智能体（例如"使用子智能体并行化"）。' },
};

export default collaboration;