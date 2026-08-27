/**
 * 按领域拆分的 IPC 封装 barrel。
 *
 * `src/utils/commands.ts` 保留为公共兼容再导出 —— 新代码推荐直接从
 * `@/utils/commands/{domain}` 导入。
 */
export * from './health';
export * from './agent';
export * from './approvals';
export * from './plan';
export * from './goal';
export * from './permissions';
export * from './questions';
export * from './config';
export * from './sessions';
export * from './events';
export * from './workspaces';
export * from './skills';
export * from './memory';
export * from './hooks';
export * from './tasks';
export * from './teams';
export * from './schedule';
export * from './agents';
export * from './side_channel';
export * from './remote';
export * from './git';
export * from './terminal';
export * from './files';
export * from './allowlist';
export * from './updates';
export * from './search';
export * from './kms';
export * from './autopilot';
export * from './activity';
export * from './squad';
export * from './media';
