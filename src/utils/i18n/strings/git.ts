/**
 * i18n namespace —— git.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const git: Record<string, StringEntry> = {
  'git.title':          { en: 'Git',                                                                                                           'zh-CN': 'Git' },
  'git.status':         { en: 'Status',                                                                                                        'zh-CN': '状态' },
  'git.diff':           { en: 'Diff',                                                                                                          'zh-CN': 'Diff' },
  'git.log':            { en: 'Log',                                                                                                           'zh-CN': '日志' },
  'git.branch':         { en: 'Branch',                                                                                                        'zh-CN': '分支' },
  'git.commit':         { en: 'Commit',                                                                                                        'zh-CN': '提交' },
  'git.push':           { en: 'Push',                                                                                                          'zh-CN': '推送' },
  'git.pull':           { en: 'Pull',                                                                                                          'zh-CN': '拉取' },
  'git.clean':          { en: 'Clean',                                                                                                         'zh-CN': '干净' },
  'git.modified':       { en: 'Modified',                                                                                                      'zh-CN': '已修改' },
  'git.added':          { en: 'Added',                                                                                                         'zh-CN': '已新增' },
  'git.deleted':        { en: 'Deleted',                                                                                                       'zh-CN': '已删除' },
  'git.untracked':      { en: 'Untracked',                                                                                                     'zh-CN': '未跟踪' },
  'git.noChanges':      { en: 'No changes.',                                                                                                   'zh-CN': '没有改动。' },
  'git.noCommits':      { en: 'No commits yet.',                                                                                               'zh-CN': '暂无提交。' },
  'git.refresh':        { en: 'Refresh',                                                                                                       'zh-CN': '刷新' },
  'git.subtitle':       { en: 'Working tree, diff, and recent log.',                                                                           'zh-CN': '工作区、diff 和最近日志。' },
  'git.notRepo':        { en: 'No git repository found.',                                                                                      'zh-CN': '未找到 git 仓库。' },
  'git.notRepoDesc':    { en: 'The current workspace is not a git repository. Run git init or open a folder that contains a .git directory.',  'zh-CN': '当前工作区不是 git 仓库。运行 git init 或打开一个包含 .git 目录的文件夹。' },
  'git.ahead':          { en: 'ahead',                                                                                                         'zh-CN': '领先' },
  'git.behind':         { en: 'behind',                                                                                                        'zh-CN': '落后' },
  'git.working':        { en: 'Working',                                                                                                       'zh-CN': '工作区' },
  'git.staged':         { en: 'Staged',                                                                                                        'zh-CN': '已暂存' },
  'git.changes':        { en: 'Changes',                                                                                                       'zh-CN': '改动' },
  'git.diffTitle':      { en: 'Diff ({tab})',                                                                                                  'zh-CN': 'Diff ({tab})' },
  'git.noDiff':         { en: 'No diff.',                                                                                                      'zh-CN': '没有 diff。' },
  'git.commits':        { en: 'Recent commits',                                                                                                'zh-CN': '最近提交' },
  'git.refreshing':     { en: 'Refreshing…',                                                                                                   'zh-CN': '正在刷新…' },
  'git.commitMessage':  { en: 'Commit message',                                                                                                'zh-CN': '提交信息' },
  'git.commitStaged':   { en: 'Commit staged',                                                                                                 'zh-CN': '提交已暂存' },
  'git.commitAll':      { en: 'Commit all',                                                                                                    'zh-CN': '提交全部' },
  'git.stageAll':       { en: 'Stage all',                                                                                                     'zh-CN': '全部暂存' },
  'git.unstageAll':     { en: 'Unstage all',                                                                                                   'zh-CN': '取消全部暂存' },
};

export default git;
