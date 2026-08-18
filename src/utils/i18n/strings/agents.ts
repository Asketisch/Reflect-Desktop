/**
 * i18n 命名空间 - agents.*
 */
import type { StringEntry } from '../types';

const agents: Record<string, StringEntry> = {
  'agents.title':                  { en: 'Agents',                                       'zh-CN': '智能体' },
  'agents.subtitle':              { en: 'Agent profiles — model, system prompt, tools. Stored at ~/.reflect/agents/*.md.', 'zh-CN': '智能体配置文件 — 模型、系统提示、工具。存储于 ~/.reflect/agents/*.md。' },
  'agents.new':                    { en: 'New',                                          'zh-CN': '新建' },
  'agents.failed':                 { en: 'Failed to load agents',                        'zh-CN': '加载智能体失败' },
  'agents.failedDesc':            { en: 'Check the agent backend and try again.',        'zh-CN': '检查后端并重试。' },
  'agents.empty':                  { en: 'No agents defined',                            'zh-CN': '暂无智能体' },
  'agents.emptyDesc':              { en: 'Create an agent profile to get started.',      'zh-CN': '创建智能体配置以开始使用。' },
  'agents.model':                  { en: 'Model',                                        'zh-CN': '模型' },
  'agents.prompt':                 { en: 'Prompt',                                       'zh-CN': '提示' },
  'agents.tools':                  { en: 'Tools',                                        'zh-CN': '工具' },
  'agents.edit':                   { en: 'Edit',                                         'zh-CN': '编辑' },
  'agents.delete':                 { en: 'Delete',                                       'zh-CN': '删除' },
  'agents.readonly':               { en: 'readonly',                                     'zh-CN': '只读' },
  'agents.spawnable':              { en: 'spawnable',                                    'zh-CN': '可派生' },
  'agents.newAgent':               { en: 'New agent',                                    'zh-CN': '新建智能体' },
  'agents.editAgent':              { en: 'Edit "{name}"',                                'zh-CN': '编辑 "{name}"' },
  'agents.cancel':                 { en: 'Cancel',                                       'zh-CN': '取消' },
  'agents.save':                   { en: 'Save',                                         'zh-CN': '保存' },
  'agents.name':                   { en: 'Name',                                         'zh-CN': '名称' },
  'agents.namePlaceholder':        { en: 'e.g. code-reviewer',                           'zh-CN': '例如 code-reviewer' },
  'agents.description':            { en: 'Description',                                  'zh-CN': '描述' },
  'agents.descriptionPlaceholder': { en: 'Short human-readable description',             'zh-CN': '简短的描述' },
  'agents.modelPlaceholder':       { en: 'inherit (default), or provider/model',         'zh-CN': '继承默认，或 provider/model' },
  'agents.toolsLabel':             { en: 'Tools (comma-separated; empty = all builtins)', 'zh-CN': '工具（逗号分隔；空 = 所有内置）' },
  'agents.toolsPlaceholder':       { en: 'read, grep, glob',                             'zh-CN': 'read, grep, glob' },
  'agents.disallowedToolsLabel':   { en: 'Disallowed tools (comma-separated)',           'zh-CN': '禁止的工具（逗号分隔）' },
  'agents.disallowedToolsPlaceholder': { en: 'bash, edit',                               'zh-CN': 'bash, edit' },
  'agents.spawnableLabel':         { en: 'Spawnable (other agents may invoke)',          'zh-CN': '可派生（其他智能体可调用）' },
  'agents.readonlyLabel':          { en: 'Readonly (no write/edit tools)',               'zh-CN': '只读（无写入/编辑工具）' },
  'agents.maxTurns':               { en: 'Max turns',                                    'zh-CN': '最大轮数' },
  'agents.unset':                  { en: 'unset',                                        'zh-CN': '未设置' },
  'agents.memoryScopes':           { en: 'Memory scopes',                                'zh-CN': '记忆范围' },
  'agents.systemPromptLabel':      { en: 'System prompt (markdown body)',                'zh-CN': '系统提示（markdown 正文）' },
  'agents.systemPromptPlaceholder': { en: 'You are a strict code reviewer...',           'zh-CN': '你是一个严格的代码审查员...' },
} as const;

export default agents;