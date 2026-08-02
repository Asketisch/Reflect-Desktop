/**
 * i18n namespace - schedule.*
 */
import type { StringEntry } from '../types';

const schedule: Record<string, StringEntry> = {
  'schedule.title':                { en: 'Schedule',             'zh-CN': '定时任务' },
  'schedule.subtitle':            { en: 'Cron-driven autonomous triggers. Due jobs fire their prompt into the agent loop.', 'zh-CN': '由 cron 驱动的自动触发器。到期任务将 prompt 注入 agent 循环。' },
  'schedule.add':                  { en: 'Add',                  'zh-CN': '添加' },
  'schedule.cancel':               { en: 'Cancel',               'zh-CN': '取消' },
  'schedule.create':               { en: 'Schedule',             'zh-CN': '创建' },
  'schedule.failed':               { en: 'Failed to load schedules', 'zh-CN': '加载定时任务失败' },
  'schedule.failedDesc':          { en: 'Check the agent backend and try again.', 'zh-CN': '检查后端并重试。' },
  'schedule.empty':                { en: 'No scheduled jobs',    'zh-CN': '暂无定时任务' },
  'schedule.emptyDesc':            { en: 'Add a cron job above to trigger the agent on a schedule.', 'zh-CN': '在上方添加 cron 任务以触发 agent。' },
  'schedule.cron':                 { en: 'Schedule (cron: min hour dom month dow, e.g. 0 9 * * 1-5)', 'zh-CN': '计划 (cron: 分 时 日 月 周, 如 0 9 * * 1-5)' },
  'schedule.prompt':               { en: 'Prompt (required)',    'zh-CN': '提示 (必填)' },
  'schedule.name':                 { en: 'Name (optional)',      'zh-CN': '名称 (选填)' },
  'schedule.nextFire':             { en: 'next fire (UTC)',      'zh-CN': '下次触发 (UTC)' },
  'schedule.remove':               { en: 'Remove',               'zh-CN': '移除' },
  'schedule.toggle':               { en: 'toggle',               'zh-CN': '切换' },
  'schedule.enabled':              { en: 'enabled',              'zh-CN': '已启用' },
  'schedule.disabled':             { en: 'disabled',             'zh-CN': '已禁用' },
  'schedule.disable':              { en: 'Disable',              'zh-CN': '禁用' },
  'schedule.enable':               { en: 'Enable',               'zh-CN': '启用' },
  'schedule.active':               { en: 'active',               'zh-CN': '活跃' },
} as const;

export default schedule;