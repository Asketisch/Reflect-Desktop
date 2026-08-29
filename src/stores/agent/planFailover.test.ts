import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  AUTO_FAILOVER_PREF_KEY,
  createPlanFailoverController,
  extractExhaustionSignal,
  FAILOVER_DEBOUNCE_MS,
  type PlanFailoverDeps,
} from './planFailover';

function makeDeps(overrides: Partial<PlanFailoverDeps> = {}) {
  const calls: { toasts: Array<{ kind: string; message: string }>; saved: string[]; switched: Array<[string, string]> } = {
    toasts: [],
    saved: [],
    switched: [],
  };
  const deps: PlanFailoverDeps = {
    getConfig: vi.fn(async () => CONFIG_TWO_PROVIDERS),
    saveConfig: vi.fn(async (toml: string) => {
      calls.saved.push(toml);
    }),
    pushToast: vi.fn((input: { kind: string; message: string }) => {
      calls.toasts.push(input);
    }),
    isEnabled: () => true,
    onSwitched: (from, to) => calls.switched.push([from, to]),
    currentProvider: () => 'anthropic',
    ...overrides,
  };
  return { deps, calls };
}

const CONFIG_TWO_PROVIDERS = `
[active]
provider = "anthropic"

[anthropic]
api_key = "sk-a"

[[anthropic.credentials]]
label = "glm"
api_key = "sk-glm"
quota = { window_secs = 3600, max_tokens = 1000, check_via = "zhipu" }

[openai]
api_key = "sk-oai"
`;

describe('extractExhaustionSignal', () => {
  it('quota_exhausted 事件直接判定为耗尽', () => {
    const signal = extractExhaustionSignal(
      { type: 'quota_exhausted', provider: 'anthropic', label: 'glm', message: 'window exhausted' },
      'anthropic',
    );
    expect(signal).toEqual({
      provider: 'anthropic',
      label: 'glm',
      reason: 'window exhausted',
    });
  });

  it('error 事件命中耗尽特征文本才判定(provider 取自 details)', () => {
    const signal = extractExhaustionSignal(
      {
        type: 'error',
        code: 'RATE_LIMITED',
        message: 'ALL_CREDENTIALS_EXHAUSTED',
        details: '{"provider":"anthropic","tried":[]}',
      },
      'anthropic',
    );
    expect(signal).not.toBeNull();
    expect(signal!.provider).toBe('anthropic');
  });

  it('普通 error(网络/认证)不判定为耗尽', () => {
    expect(
      extractExhaustionSignal({ type: 'error', code: 'AUTH_FAILED', message: 'invalid api key' }, 'anthropic'),
    ).toBeNull();
    expect(
      extractExhaustionSignal({ type: 'error', code: 'NETWORK', message: 'connection reset' }, 'anthropic'),
    ).toBeNull();
    expect(extractExhaustionSignal({ type: 'stream_error' }, 'anthropic')).toBeNull();
  });
});

describe('createPlanFailoverController', () => {
  afterEach(() => {
    localStorage.removeItem(AUTO_FAILOVER_PREF_KEY);
    vi.restoreAllMocks();
  });

  it('耗尽时切换到备用 provider 并落盘 + toast + 记录', async () => {
    const { deps, calls } = makeDeps();
    const controller = createPlanFailoverController(deps);
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    expect(calls.saved).toHaveLength(1);
    expect(calls.saved[0]).toContain('provider = "openai"');
    expect(calls.switched).toEqual([['anthropic', 'openai']]);
    expect(calls.toasts[0].kind).toBe('success');
  });

  it('无备用计划时只弹提示,不改配置', async () => {
    const { deps, calls } = makeDeps({
      getConfig: vi.fn(async () => '[anthropic]\napi_key = "sk-a"\n'),
    });
    const controller = createPlanFailoverController(deps);
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    expect(calls.saved).toHaveLength(0);
    expect(calls.toasts[0].kind).toBe('error');
    expect(calls.switched).toEqual([]);
  });

  it('防抖窗口内不重复切换', async () => {
    vi.useFakeTimers();
    try {
      const { deps, calls } = makeDeps();
      const controller = createPlanFailoverController(deps);
      await controller.handle({ provider: 'anthropic', reason: 'quota' });
      expect(calls.saved).toHaveLength(1);
      // 同一防抖窗口内再次耗尽 → 不切换。
      await controller.handle({ provider: 'openai', reason: 'quota' });
      expect(calls.saved).toHaveLength(1);
      // 超过防抖窗口后可再次切换。
      vi.setSystemTime(Date.now() + FAILOVER_DEBOUNCE_MS + 1);
      await controller.handle({ provider: 'anthropic', reason: 'quota' });
      expect(calls.saved).toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it('开关关闭时不做任何事', async () => {
    const { deps, calls } = makeDeps({ isEnabled: () => false });
    const controller = createPlanFailoverController(deps);
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    expect(calls.saved).toHaveLength(0);
    expect(calls.toasts).toHaveLength(0);
  });

  it('reset 清空防抖状态', async () => {
    const { deps, calls } = makeDeps();
    const controller = createPlanFailoverController(deps);
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    controller.reset();
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    expect(calls.saved).toHaveLength(2);
  });

  it('config 保存失败时弹错误提示', async () => {
    const { deps, calls } = makeDeps({
      saveConfig: vi.fn(async () => {
        throw new Error('validation failed');
      }),
    });
    const controller = createPlanFailoverController(deps);
    await controller.handle({ provider: 'anthropic', reason: 'quota' });
    expect(calls.toasts[0].kind).toBe('error');
    expect(calls.switched).toEqual([]);
  });

  it('isAutoFailoverEnabled:缺省开启,localStorage 设 off 后关闭', async () => {
    localStorage.removeItem(AUTO_FAILOVER_PREF_KEY);
    const { isAutoFailoverEnabled } = await import('./planFailover');
    expect(isAutoFailoverEnabled()).toBe(true);
    localStorage.setItem(AUTO_FAILOVER_PREF_KEY, 'off');
    expect(isAutoFailoverEnabled()).toBe(false);
  });
});
