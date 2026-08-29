/**
 * i18n 命名空间 - squad.*
 */
import type { StringEntry } from '../types';

const squad: Record<string, StringEntry> = {
  'squad.title':                 { en: 'Squad', 'zh-CN': '小队' },
  'squad.subtitle':              { en: 'Multi-agent teams with leader delegation.', 'zh-CN': '支持 leader 委派的多 agent 团队。' },
  'squad.squads':                { en: 'Squads', 'zh-CN': '小队列表' },
  'squad.empty':                 { en: 'No squads yet', 'zh-CN': '暂无小队' },
  'squad.emptyDesc':             { en: 'Create one below to start coordinating multi-agent work.', 'zh-CN': '在下方创建一个小队，开始协同完成多 agent 任务。' },
  'squad.memberCount':           { en: '{count} member{plural}', 'zh-CN': '{count} 名成员' },
  'squad.createSquad':           { en: 'Create squad', 'zh-CN': '创建小队' },
  'squad.namePlaceholder':       { en: 'squad-name (a-z0-9_-)', 'zh-CN': 'squad-name (a-z0-9_-)' },
  'squad.descriptionPlaceholder':{ en: 'Description (optional)', 'zh-CN': '描述 (选填)' },
  'squad.create':                { en: 'Create', 'zh-CN': '创建' },
  'squad.selectTitle':           { en: 'Select a squad', 'zh-CN': '选择一个小队' },
  'squad.selectDesc':            { en: 'Pick a squad from the list to view members and delegate tasks.', 'zh-CN': '从列表中选择一个小队，查看成员并委派任务。' },
  'squad.leaderBadge':           { en: 'leader: {id}', 'zh-CN': 'leader: {id}' },
  'squad.membersCount':          { en: 'Members ({count})', 'zh-CN': '成员 ({count})' },
  'squad.noMembers':             { en: 'No additional members; the lead operates solo.', 'zh-CN': '暂无其他成员；leader 独立运行。' },
  'squad.tasksTitle':            { en: 'Tasks in this squad', 'zh-CN': '此小队中的任务' },
  'squad.delegateNext':          { en: 'Delegate next', 'zh-CN': '委派下一个' },
  'squad.noTasksPrefix':         { en: 'No tasks yet. Create one via the Tasks board (', 'zh-CN': '暂无任务。可在任务看板中创建（' },
  'squad.noTasksSuffix':         { en: ').', 'zh-CN': '）。' },
  'squad.owner':                 { en: 'owner: {name}', 'zh-CN': '负责人: {name}' },
  'squad.unassigned':            { en: '— unassigned —', 'zh-CN': '— 未分配 —' },
  'squad.assign':                { en: 'Assign', 'zh-CN': '分配' },
  'squad.rolePlaceholder':       { en: 'role (e.g. architect)', 'zh-CN': '角色 (如 architect)' },
  'squad.addMember':             { en: 'Add member', 'zh-CN': '添加成员' },
  'squad.toastCreated':          { en: 'Squad "{name}" created', 'zh-CN': '小队「{name}」已创建' },
  'squad.toastDeleted':          { en: 'Squad deleted', 'zh-CN': '小队已删除' },
  'squad.toastClaimed':          { en: 'Leader claimed task #{id}', 'zh-CN': 'leader 已认领任务 #{id}' },
  'squad.toastNoPending':        { en: 'No pending task available', 'zh-CN': '没有可认领的待处理任务' },
  'squad.toastAssigned':         { en: 'Assigned to {assignee}', 'zh-CN': '已分配给 {assignee}' },
  'squad.toastUnassigned':       { en: 'Assignment cleared', 'zh-CN': '已清空分配' },
} as const;

export default squad;
