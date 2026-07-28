/**
 * Domain-split barrel for IPC wrappers.
 *
 * `src/utils/commands.ts` remains the public compatibility re-export —
 * prefer importing from `@/utils/commands/{domain}` directly in new code.
 */
export * from './health';
export * from './agent';
export * from './approvals';
export * from './plan';
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
