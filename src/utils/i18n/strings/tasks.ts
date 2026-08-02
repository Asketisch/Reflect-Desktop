/**
 * i18n namespace - tasks.*
 */
import type { StringEntry } from '../types';

const tasks: Record<string, StringEntry> = {
  'tasks.title':                    { en: 'Tasks',              'zh-CN': '任务' },
  'tasks.subtitle':                { en: 'Multi-agent task board. Create, claim, advance, and delete tasks across lists.', 'zh-CN': '多智能体任务看板。在列表间创建、认领、推进和删除任务。' },
  'tasks.listView':                { en: 'List',               'zh-CN': '列表' },
  'tasks.boardView':               { en: 'Board',              'zh-CN': '看板' },
  'tasks.add':                     { en: 'Add',                'zh-CN': '添加' },
  'tasks.cancel':                  { en: 'Cancel',             'zh-CN': '取消' },
  'tasks.create':                  { en: 'Create',             'zh-CN': '创建' },
  'tasks.claim':                   { en: 'Claim',              'zh-CN': '认领' },
  'tasks.start':                   { en: 'Start',              'zh-CN': '开始' },
  'tasks.complete':                { en: 'Complete',           'zh-CN': '完成' },
  'tasks.delete':                  { en: 'Delete',             'zh-CN': '删除' },
  'tasks.pending':                 { en: 'Pending',            'zh-CN': '待处理' },
  'tasks.in_progress':             { en: 'In Progress',        'zh-CN': '进行中' },
  'tasks.completed':               { en: 'Completed',          'zh-CN': '已完成' },
  'tasks.deleted':                 { en: 'Deleted',            'zh-CN': '已删除' },
  'tasks.failed':                  { en: 'Failed to load tasks', 'zh-CN': '加载任务失败' },
  'tasks.failedDesc':              { en: 'Check the agent backend and try again.', 'zh-CN': '检查后端并重试。' },
  'tasks.empty':                   { en: 'No tasks',           'zh-CN': '暂无任务' },
  'tasks.emptyDesc':               { en: 'List "{listId}" is empty. Add one above.', 'zh-CN': '「{listId}」列表为空。在上方添加任务。' },
  'tasks.subject':                 { en: 'Subject (required)', 'zh-CN': '标题 (必填)' },
  'tasks.description':             { en: 'Description (optional)', 'zh-CN': '描述 (选填)' },
  'tasks.owner':                   { en: 'Owner / claimer (optional)', 'zh-CN': '负责人 / 认领人 (选填)' },
  'tasks.switchList':              { en: 'Switch list / team', 'zh-CN': '切换列表 / 团队' },
  'tasks.activeList':              { en: 'Active list',        'zh-CN': '当前列表' },
  'tasks.listPlaceholder':         { en: 'list id (e.g. default, or a team name)', 'zh-CN': '列表 ID (如 default 或团队名)' },
  'tasks.teams':                   { en: 'Teams',              'zh-CN': '团队' },
  'tasks.claimed_by':              { en: 'claimed by',         'zh-CN': '认领人' },
} as const;

export default tasks;