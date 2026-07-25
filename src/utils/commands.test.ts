/**
 * Vitest — commands 包装测试。
 *
 * 验证 wrapper 调用 invoke 转发正确 cmd + 参数（依赖 test/setup.tsx 的 mock）。
 *
 * 第二组 `forwarding (mapping)` 用例覆盖各 domain 的代表 wrapper,
 * 确认 cmd name 与 argument object key 与历史实现完全一致 —— 拆目录后
 * 重新集中保护这条不变量。
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { mockInvoke, resetMockInvoke } from '@/test/setup';
import {
  // agent
  ping,
  reflect_submit,
  reflect_interrupt,
  reflect_compact,
  reflect_rewind,
  reflect_shutdown,
  // approvals
  reflect_tool_approval,
  reflect_hook_approval,
  reflect_plan_approval,
  // plan
  reflect_enter_plan_mode,
  reflect_exit_plan_mode,
  // permissions
  reflect_set_effort,
  reflect_set_permission_mode,
  reflect_cycle_permission_mode,
  // questions
  reflect_ask_user_question_response,
  reflect_ask_user_input_response,
  // config
  reflect_agent_status,
  reflect_get_config,
  reflect_save_config,
  reflect_list_tools,
  // sessions
  reflect_list_sessions,
  reflect_rename_session,
  reflect_delete_session,
  reflect_replay_session,
  reflect_export_session,
  reflect_export_session_markdown,
  // events
  onReflectEvent,
  // workspaces
  reflect_list_workspaces,
  reflect_set_workspace,
  reflect_current_workspace,
  // skills
  reflect_list_skills,
  // memory
  reflect_list_memory,
  reflect_add_memory,
  reflect_remove_memory,
  // hooks
  reflect_list_hooks,
  reflect_toggle_hook,
  // git
  reflect_git_status,
  reflect_git_diff,
  reflect_git_log,
  // terminal
  reflect_run_shell,
  reflect_kill_shell,
  reflect_list_shell_sessions,
  onTerminalOutput,
  // files
  reflect_list_dir,
  reflect_read_file,
  // allowlist
  reflect_load_allowlist,
  reflect_save_allowlist,
  reflect_check_allowlist,
  // updates
  reflect_check_update,
  // search
  reflect_search_files,
} from '@/utils/commands';

/**
 * Capture a single call's cmd + args.
 * Returns the recorded args object (or undefined for wrappers that pass no args).
 */
async function captureCmd<T>(cmd: string, fn: () => Promise<T>): Promise<unknown> {
  return new Promise((resolve, reject) => {
    mockInvoke(cmd, async (_c, args) => {
      resolve(args);
      return undefined;
    });
    fn().catch(reject);
  });
}

describe('commands (forwarding to mock invoke)', () => {
  it('reflect_interrupt calls invoke("reflect_interrupt")', async () => {
    // setup.tsx 已 mock reflect_set_permission_mode 等; reflect_interrupt 未注册 → mock 抛错
    await expect(reflect_interrupt()).rejects.toThrow(/not mocked/);
  });

  it('reflect_compact also un-registered by default', async () => {
    await expect(reflect_compact()).rejects.toThrow(/not mocked/);
  });

  it('reflect_rewind forwards toTurnId argument', async () => {
    await expect(reflect_rewind('turn-123')).rejects.toThrow(/not mocked/);
    await expect(reflect_rewind()).rejects.toThrow(/not mocked/);
  });

  it('reflect_tool_approval accepts ReviewDecision', async () => {
    await expect(reflect_tool_approval('id-1', 'approve')).rejects.toThrow(/not mocked/);
    await expect(
      reflect_tool_approval('id-2', { deny: { reason: 'risky' } }),
    ).rejects.toThrow(/not mocked/);
    await expect(
      reflect_tool_approval('id-3', 'approve_for_session'),
    ).rejects.toThrow(/not mocked/);
  });

  it('reflect_set_permission_mode is mocked by setup and returns undefined', async () => {
    expect(await reflect_set_permission_mode('default')).toBeUndefined();
  });
});

describe('commands forwarding (mapping)', () => {
  beforeEach(() => {
    resetMockInvoke();
  });

  /** Compare against the literal record passed by the wrapper. */
  function expectArgs(actual: unknown, expected: Record<string, unknown> | undefined) {
    expect(actual).toEqual(expected);
  }

  // ----- ping -----

  it('ping → invoke("ping") with no args', async () => {
    const args = await captureCmd('ping', ping);
    expectArgs(args, undefined);
  });

  // ----- agent lifecycle -----

  it('reflect_submit forwards { submission }', async () => {
    const args = await captureCmd('reflect_submit', () =>
      reflect_submit({ id: 'sub-1', op: { type: 'interrupt' } } as never),
    );
    expectArgs(args, { submission: { id: 'sub-1', op: { type: 'interrupt' } } });
  });

  it('reflect_interrupt / compact / shutdown use bare cmd names', async () => {
    const seenArgs: Array<{ cmd: string; args: unknown }> = [];
    for (const [cmd, fn] of [
      ['reflect_interrupt', reflect_interrupt],
      ['reflect_compact', reflect_compact],
      ['reflect_shutdown', reflect_shutdown],
    ] as const) {
      mockInvoke(cmd, async (c, args) => {
        seenArgs.push({ cmd: c, args });
        return c === 'reflect_interrupt' ? undefined : `sub-${c}`;
      });
      await fn();
    }
    expect(seenArgs).toEqual([
      { cmd: 'reflect_interrupt', args: undefined },
      { cmd: 'reflect_compact', args: undefined },
      { cmd: 'reflect_shutdown', args: undefined },
    ]);
  });

  it('reflect_rewind encodes `toTurnId` (camelCase) and normalises undefined → null', async () => {
    const withId = await captureCmd('reflect_rewind', () => reflect_rewind('turn-xyz'));
    expectArgs(withId, { toTurnId: 'turn-xyz' });

    const noId = await captureCmd('reflect_rewind', () => reflect_rewind());
    expectArgs(noId, { toTurnId: null });
  });

  // ----- approvals -----

  it('each approval forwards { id, decision } identically', async () => {
    const cases = [
      ['reflect_tool_approval', reflect_tool_approval],
      ['reflect_hook_approval', reflect_hook_approval],
      ['reflect_plan_approval', reflect_plan_approval],
    ] as const;
    for (const [cmd, fn] of cases) {
      const captured = await captureCmd(cmd, () => fn('approval-42', 'approve'));
      expectArgs(captured, { id: 'approval-42', decision: 'approve' });
    }
  });

  // ----- plan -----

  it('reflect_enter_plan_mode forwards { task }; exit_plan_mode has none', async () => {
    const enter = await captureCmd('reflect_enter_plan_mode', () =>
      reflect_enter_plan_mode('plan this'),
    );
    expectArgs(enter, { task: 'plan this' });

    const exit = await captureCmd('reflect_exit_plan_mode', () => reflect_exit_plan_mode());
    expectArgs(exit, undefined);
  });

  // ----- permissions -----

  it('reflect_set_effort forwards { level }; set_permission_mode { mode }; cycle has none', async () => {
    const effort = await captureCmd('reflect_set_effort', () => reflect_set_effort('high'));
    expectArgs(effort, { level: 'high' });

    const perm = await captureCmd('reflect_set_permission_mode', () =>
      reflect_set_permission_mode('auto'),
    );
    expectArgs(perm, { mode: 'auto' });

    const cycle = await captureCmd('reflect_cycle_permission_mode', () =>
      reflect_cycle_permission_mode(),
    );
    expectArgs(cycle, undefined);
  });

  // ----- questions -----

  it('reflect_ask_user_question_response forwards { id, answers }', async () => {
    const args = await captureCmd('reflect_ask_user_question_response', () =>
      reflect_ask_user_question_response('q-1', { q1: 'a' }),
    );
    expectArgs(args, { id: 'q-1', answers: { q1: 'a' } });
  });

  it('reflect_ask_user_input_response forwards { id, text }', async () => {
    const args = await captureCmd('reflect_ask_user_input_response', () =>
      reflect_ask_user_input_response('q-2', 'typed answer'),
    );
    expectArgs(args, { id: 'q-2', text: 'typed answer' });
  });

  // ----- config / agent status -----

  it('reflect_agent_status / get_config / list_tools use bare cmd names', async () => {
    const seenArgs: Array<{ cmd: string; args: unknown }> = [];
    for (const cmd of ['reflect_agent_status', 'reflect_get_config', 'reflect_list_tools']) {
      mockInvoke(cmd, async (c, args) => {
        seenArgs.push({ cmd: c, args });
        return c === 'reflect_list_tools' ? [] : '';
      });
    }
    await reflect_agent_status();
    await reflect_get_config();
    await reflect_list_tools();
    expect(seenArgs).toEqual([
      { cmd: 'reflect_agent_status', args: undefined },
      { cmd: 'reflect_get_config', args: undefined },
      { cmd: 'reflect_list_tools', args: undefined },
    ]);
  });

  it('reflect_save_config forwards { toml }', async () => {
    const args = await captureCmd('reflect_save_config', () =>
      reflect_save_config('foo = 1\n'),
    );
    expectArgs(args, { toml: 'foo = 1\n' });
  });

  // ----- sessions -----

  it('reflect_list_sessions normalises missing opts to { limit: null, offset: null }', async () => {
    const none = await captureCmd('reflect_list_sessions', () => reflect_list_sessions());
    expectArgs(none, { limit: null, offset: null });

    const some = await captureCmd('reflect_list_sessions', () =>
      reflect_list_sessions({ limit: 10, offset: 20 }),
    );
    expectArgs(some, { limit: 10, offset: 20 });
  });

  it('reflect_rename_session encodes newName (camelCase); delete forwards { id } only', async () => {
    const rename = await captureCmd('reflect_rename_session', () =>
      reflect_rename_session('sess-1', 'My session'),
    );
    expectArgs(rename, { id: 'sess-1', newName: 'My session' });

    const del = await captureCmd('reflect_delete_session', () =>
      reflect_delete_session('sess-2'),
    );
    expectArgs(del, { id: 'sess-2' });
  });

  it('reflect_replay_session / export_session / export_session_markdown forward { id }', async () => {
    const cases: Array<[string, (id: string) => Promise<unknown>]> = [
      ['reflect_replay_session', reflect_replay_session],
      ['reflect_export_session', reflect_export_session],
      ['reflect_export_session_markdown', reflect_export_session_markdown],
    ];
    for (const [cmd, fn] of cases) {
      const args = await captureCmd(cmd, () => fn('sess-9'));
      expectArgs(args, { id: 'sess-9' });
    }
  });

  // ----- events -----

  it('onReflectEvent listens on "reflect_event" and unwraps .payload', async () => {
    const unlisten = await onReflectEvent(() => {});
    expect(typeof unlisten).toBe('function');
    unlisten();
  });

  // ----- workspaces -----

  it('reflect_set_workspace forwards { path }; other workspace cmds have no args', async () => {
    const set = await captureCmd('reflect_set_workspace', () =>
      reflect_set_workspace('/tmp/proj'),
    );
    expectArgs(set, { path: '/tmp/proj' });

    const seenArgs: Array<{ cmd: string; args: unknown }> = [];
    for (const cmd of ['reflect_list_workspaces', 'reflect_current_workspace']) {
      mockInvoke(cmd, async (c, args) => {
        seenArgs.push({ cmd: c, args });
        return c === 'reflect_list_workspaces' ? [] : '/tmp/proj';
      });
    }
    await reflect_list_workspaces();
    await reflect_current_workspace();
    expect(seenArgs).toEqual([
      { cmd: 'reflect_list_workspaces', args: undefined },
      { cmd: 'reflect_current_workspace', args: undefined },
    ]);
  });

  // ----- skills -----

  it('reflect_list_skills has no args', async () => {
    const args = await captureCmd('reflect_list_skills', () => reflect_list_skills());
    expectArgs(args, undefined);
  });

  // ----- memory -----

  it('memory wrappers forward { scope, key, value? }', async () => {
    const list = await captureCmd('reflect_list_memory', () => reflect_list_memory());
    expectArgs(list, undefined);

    const add = await captureCmd('reflect_add_memory', () =>
      reflect_add_memory('user', 'name', 'Alice'),
    );
    expectArgs(add, { scope: 'user', key: 'name', value: 'Alice' });

    const rm = await captureCmd('reflect_remove_memory', () =>
      reflect_remove_memory('user', 'name'),
    );
    expectArgs(rm, { scope: 'user', key: 'name' });
  });

  // ----- hooks -----

  it('reflect_toggle_hook forwards { name, enabled }; list_hooks has no args', async () => {
    const toggle = await captureCmd('reflect_toggle_hook', () =>
      reflect_toggle_hook('pre-commit', true),
    );
    expectArgs(toggle, { name: 'pre-commit', enabled: true });

    const list = await captureCmd('reflect_list_hooks', () => reflect_list_hooks());
    expectArgs(list, undefined);
  });

  // ----- git -----

  it('reflect_git_status has no args; diff forwards { staged }; log forwards { limit }', async () => {
    const status = await captureCmd('reflect_git_status', () => reflect_git_status());
    expectArgs(status, undefined);

    const diff = await captureCmd('reflect_git_diff', () => reflect_git_diff(true));
    expectArgs(diff, { staged: true });

    const log = await captureCmd('reflect_git_log', () => reflect_git_log(50));
    expectArgs(log, { limit: 50 });
  });

  // ----- terminal -----

  it('terminal spawn forwards { cmd }; kill forwards snake_case { session_id }; list has no args', async () => {
    const run = await captureCmd('reflect_run_shell', () => reflect_run_shell('ls -la'));
    expectArgs(run, { cmd: 'ls -la' });

    const kill = await captureCmd('reflect_kill_shell', () => reflect_kill_shell('sh-1'));
    // backend key is snake_case "session_id" — preserve that for compat.
    expectArgs(kill, { session_id: 'sh-1' });

    const list = await captureCmd('reflect_list_shell_sessions', () =>
      reflect_list_shell_sessions(),
    );
    expectArgs(list, undefined);
  });

  it('onTerminalOutput listens on "reflect_terminal_output" and unwraps .payload', async () => {
    const unlisten = await onTerminalOutput(() => {});
    expect(typeof unlisten).toBe('function');
    unlisten();
  });

  // ----- files -----

  it('reflect_list_dir forwards { path, maxDepth }; read_file forwards { path }', async () => {
    const dir = await captureCmd('reflect_list_dir', () => reflect_list_dir(null));
    expectArgs(dir, { path: null, maxDepth: 4 });

    const dirScoped = await captureCmd('reflect_list_dir', () =>
      reflect_list_dir('/some/dir', 2),
    );
    expectArgs(dirScoped, { path: '/some/dir', maxDepth: 2 });

    const read = await captureCmd('reflect_read_file', () => reflect_read_file('/a.ts'));
    expectArgs(read, { path: '/a.ts' });
  });

  // ----- allowlist -----

  it('reflect_load_allowlist has no args; save_allowlist forwards { list }; check_allowlist forwards { prefix }', async () => {
    const load = await captureCmd('reflect_load_allowlist', () => reflect_load_allowlist());
    expectArgs(load, undefined);

    const list = { prefixes: ['rm -rf', 'sudo '] };
    const save = await captureCmd('reflect_save_allowlist', () =>
      reflect_save_allowlist(list),
    );
    expectArgs(save, { list });

    const check = await captureCmd('reflect_check_allowlist', () =>
      reflect_check_allowlist('rm '),
    );
    expectArgs(check, { prefix: 'rm ' });
  });

  // ----- updates -----

  it('reflect_check_update has no args', async () => {
    const args = await captureCmd('reflect_check_update', () => reflect_check_update());
    expectArgs(args, undefined);
  });

  // ----- search -----

  it('reflect_search_files forwards { query, path, maxResults } with defaults', async () => {
    const none = await captureCmd('reflect_search_files', () => reflect_search_files('foo'));
    expectArgs(none, { query: 'foo', path: null, maxResults: 200 });

    const some = await captureCmd('reflect_search_files', () =>
      reflect_search_files('foo', '/tmp', 50),
    );
    expectArgs(some, { query: 'foo', path: '/tmp', maxResults: 50 });
  });
});
