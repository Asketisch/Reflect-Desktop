/**
 * i18n 命名空间 - autopilot.*
 */
import type { StringEntry } from '../types';

const autopilot: Record<string, StringEntry> = {
  'autopilot.title':                 { en: 'Autopilot', 'zh-CN': '自动任务' },
  'autopilot.subtitle':              { en: 'Automatic task scheduling. Create and execute tasks on a cron schedule.', 'zh-CN': '自动任务调度。创建并执行基于 cron 计划的任务。' },
  'autopilot.configuration':         { en: 'Configuration', 'zh-CN': '配置' },
  'autopilot.enabled':               { en: 'Enabled', 'zh-CN': '已启用' },
  'autopilot.disabled':              { en: 'Disabled', 'zh-CN': '已禁用' },
  'autopilot.enable':                { en: 'Enable autopilot', 'zh-CN': '启用自动任务' },
  'autopilot.cronSchedule':          { en: 'Cron schedule', 'zh-CN': 'Cron 计划' },
  'autopilot.cronPlaceholder':       { en: '0 9 * * 1-5 (weekdays at 9am)', 'zh-CN': '0 9 * * 1-5（工作日每天上午 9 点）' },
  'autopilot.taskTemplate':          { en: 'Task template', 'zh-CN': '任务模板' },
  'autopilot.taskTemplatePlaceholder': { en: 'Prompt template for automatically created tasks...', 'zh-CN': '自动创建任务的 Prompt 模板…' },
  'autopilot.agent':                 { en: 'Agent', 'zh-CN': 'Agent' },
  'autopilot.agentPlaceholder':      { en: 'default (optional)', 'zh-CN': 'default（选填）' },
  'autopilot.maxConcurrent':         { en: 'Max concurrent', 'zh-CN': '最大并发' },
  'autopilot.edit':                  { en: 'Edit', 'zh-CN': '编辑' },
  'autopilot.runHistory':            { en: 'Run History', 'zh-CN': '运行历史' },
  'autopilot.noRuns':                { en: 'No runs yet', 'zh-CN': '暂无运行记录' },
  'autopilot.noRunsDesc':            { en: 'Autopilot runs will appear here once enabled.', 'zh-CN': '启用自动任务后，运行记录会显示在这里。' },
} as const;

export default autopilot;
