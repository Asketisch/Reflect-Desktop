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

  // ===== Token usage（最近一次 token_count 事件的快照）=====
  'inspector.tokenUsage':        { en: 'Token usage',                 'zh-CN': 'Token 用量' },
  'inspector.tokenEmpty':        { en: 'No token data yet.',          'zh-CN': '暂无 token 数据。' },
  'inspector.tokenInput':        { en: 'Input',                       'zh-CN': '输入' },
  'inspector.tokenOutput':       { en: 'Output',                      'zh-CN': '输出' },
  'inspector.tokenCached':       { en: 'Cached',                      'zh-CN': '缓存命中' },
  'inspector.tokenCachedHint':   { en: 'cache_read — subset of input','zh-CN': 'cache_read —— 输入的子集' },
  'inspector.tokenCacheWrite':   { en: 'Cache write',                 'zh-CN': '缓存写入' },
  'inspector.tokenCacheWriteHint': { en: 'cache_creation — subset of input, not added to total', 'zh-CN': 'cache_creation —— 输入的子集,不计入 total' },
  'inspector.tokenTotal':        { en: 'Total',                       'zh-CN': '合计' },
  'inspector.tokenCost':         { en: 'Cost',                        'zh-CN': '成本' },
  'inspector.tokenProvider':     { en: 'Provider',                    'zh-CN': '提供商' },
  'inspector.tokenCredential':   { en: 'Credential',                  'zh-CN': '凭据' },
  'inspector.contextWindow':     { en: 'Context window: {tokens} tokens', 'zh-CN': '上下文窗口: {tokens} tokens' },
};

export default inspector;
