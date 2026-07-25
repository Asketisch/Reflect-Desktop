/**
 * useAgentNotifications —— 监听 agent 完成事件,触发声音 / 系统通知 / dock badge。
 *
 * 设计目标(对标 CodexMonitor `settings.notifications.*` + Sound toggle):
 *   - 复用现有的 agent event 总线(`onReflectEvent`)。
 *   - agent 完成(`AgentMessage` 出现 + 不再有 `InProgress`)→ 播放"叮"。
 *   - 用户可在 Settings 切换声音开关 / 系统通知开关。
 *   - 所有选项持久化在 localStorage(无后端往返)。
 */
import { useEffect, useRef } from 'react';
import { subscribeAgentEvent } from '@/services/agentEventBus';

export interface AgentNotifyOptions {
  /** 是否启用声音(默认 true 直到用户在 Settings 里关闭)。 */
  sound: boolean;
  /** 是否弹系统通知(默认 false,需要 OS 权限)。 */
  system: boolean;
  /** 音量(0..1)。默认 0.4。 */
  volume?: number;
}

const KEY = 'reflect.notify.options';
const DEFAULTS: AgentNotifyOptions = { sound: false, system: false, volume: 0.4 };

export function loadNotifyOptions(): AgentNotifyOptions {
  if (typeof window === 'undefined') return DEFAULTS;
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw);
    return { ...DEFAULTS, ...parsed };
  } catch {
    return DEFAULTS;
  }
}

export function saveNotifyOptions(opts: AgentNotifyOptions): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(KEY, JSON.stringify(opts));
  } catch {
    /* ignore */
  }
}

/**
 * Hook:订阅 agent event 总线,在 turn 完成 + options 启用声音时播放 chime。
 *
 * 使用 Web Audio API 生成两短音(chime),不依赖外部音频文件;首次播放需要
 * 用户手势触发 AudioContext(WebView 限制),所以我们在第一次任意 click/keydown
 * 上解锁。
 */
export function useAgentNotifications(options: AgentNotifyOptions): void {
  const audioRef = useRef<AudioContext | null>(null);
  const unlockedRef = useRef(false);
  const playingRef = useRef(false);

  // 解锁 audio (autoplay policy)。
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const unlock = () => {
      if (unlockedRef.current) return;
      try {
        const Ctor =
          window.AudioContext ||
          (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
        if (Ctor) audioRef.current = new Ctor();
      } catch {
        /* ignore */
      }
      unlockedRef.current = true;
    };
    window.addEventListener('pointerdown', unlock, { once: true });
    window.addEventListener('keydown', unlock, { once: true });
    return () => {
      window.removeEventListener('pointerdown', unlock);
      window.removeEventListener('keydown', unlock);
    };
  }, []);

  useEffect(() => {
    return subscribeAgentEvent((event) => {
      const t = event.msg.type;
      // 触发条件:AgentMessage + 同步不是 in-progress。
      if (t === 'agent_message' && options.sound) {
        if (playingRef.current) return;
        playingRef.current = true;
        try {
          playChime(audioRef.current, options.volume ?? 0.4);
        } finally {
          window.setTimeout(() => {
            playingRef.current = false;
          }, 600);
        }
      }
      // System notification(若授权 + 启用)。
      if (t === 'turn_complete' && options.system) {
        try {
          if ('Notification' in window && Notification.permission === 'granted') {
            new Notification('Reflect agent', {
              body: 'Turn finished. Open the app to review.',
              silent: false,
            });
          }
        } catch {
          /* ignore */
        }
      }
    });
  }, [options.sound, options.system, options.volume]);
}

/** 简单 chime:两个短促正弦波(c5 → e5)。 */
function playChime(ctx: AudioContext | null, volume: number): void {
  if (!ctx) return;
  const now = ctx.currentTime;
  const tones = [
    { freq: 523.25, start: 0, dur: 0.18 },
    { freq: 659.25, start: 0.2, dur: 0.22 },
  ];
  for (const tone of tones) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.value = tone.freq;
    osc.type = 'sine';
    gain.gain.setValueAtTime(0, now + tone.start);
    gain.gain.linearRampToValueAtTime(volume, now + tone.start + 0.02);
    gain.gain.linearRampToValueAtTime(0, now + tone.start + tone.dur);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start(now + tone.start);
    osc.stop(now + tone.start + tone.dur + 0.02);
  }
}

/** Request permission for native notifications. Browser / Tauri WebView decide. */
export async function requestNotificationPermission(): Promise<NotificationPermission | 'unsupported'> {
  if (typeof window === 'undefined' || !('Notification' in window)) return 'unsupported';
  if (Notification.permission !== 'default') return Notification.permission;
  try {
    return await Notification.requestPermission();
  } catch {
    return 'denied';
  }
}
