/**
 * Side-channel 封装 —— Phase 2 第 1 项。
 *
 * 对应 `src-tauri/src/commands/side_channel.rs`(其内部封装
 * `reflect-app-core::side_channel::SideChannelRegistry`)。注册表为每条
 * side-channel 持有独立的 `CancelToken`;主 agent 的 `Cmd+C` 不会取消
 * side-channel。每条 side-channel 的状态迁移(`Started` / `Done` /
 * `Cancelled` / `Error`)通过现有的 `reflect_event` 通道(事件以
 * `kind: "side_channel_*"` 标记)推送到前端;本封装文件暴露面向页面刷新
 * 与 slash 命令的临时 CRUD 接口。
 *
 * 暂不在范围内(后续轮次):真正执行 side-channel 的 driver 任务
 * (把其 prompt 作为 `Submission::user_input` 接入 agent 循环,并在完成时
 * 更新注册表条目)。注册表与命令已经就位,可以在不改动 IPC 面的前提下
 * 后续追加 driver。
 */
import { invoke } from '../bridge';

// ── 类型(对应 `reflect_app_core::side_channel::{Status, Info}`,camelCase) ──

export type ReflectSideChannelStatus = 'running' | 'done' | 'cancelled' | 'error';

export interface ReflectSideChannelInfo {
  /** 稳定 id(`side-<8hex>`)。 */
  id: string;
  /** agent 定义名称。 */
  agentName: string;
  /** 初始 prompt。 */
  prompt: string;
  /** side-channel 注册时的 UNIX epoch 毫秒数。 */
  startedAtMs: number;
  /** 当前状态。 */
  status: ReflectSideChannelStatus;
  /** 已运行时长(毫秒);仍在运行时为 `null`。 */
  durationMs: number | null;
}

export interface ReflectStartSideChannelResult {
  id: string;
  agentName: string;
  prompt: string;
  startedAtMs: number;
}

// ── Commands ───────────────────────────────────────────────────────────

/**
 * 启动一条新的 side-channel。返回分配的 id 及快照字段。
 * 同时在注册表的 broadcast 通道上发出 `kind: "side_channel_started"` 事件
 * (由安装阶段的事件转发器并入现有的 Tauri `reflect_event` 流)。
 */
export async function reflect_start_side_channel(args: {
  agent_name: string;
  prompt: string;
}): Promise<ReflectStartSideChannelResult> {
  return invoke<ReflectStartSideChannelResult>('reflect_start_side_channel', {
    // Tauri 2 平铺参数按 camelCase 匹配 Rust 参数名 `agent_name`。
    agentName: args.agent_name,
    prompt: args.prompt,
  });
}

/**
 * 按 id 取消一条运行中的 side-channel。已生效返回 `true`,
 * 未找到 / 已处于终止态返回 `false`。
 */
export async function reflect_cancel_side_channel(id: string): Promise<boolean> {
  return invoke<boolean>('reflect_cancel_side_channel', { id });
}

/** 列出当前所有 side-channel(快照)。前端依赖事件流做实时更新,
 * 此处用于临时读取(例如页面刷新)。 */
export async function reflect_list_side_channels(): Promise<ReflectSideChannelInfo[]> {
  return invoke<ReflectSideChannelInfo[]>('reflect_list_side_channels');
}

/** 单条 side-channel 快照。id 未找到时抛错。 */
export async function reflect_get_side_channel(id: string): Promise<ReflectSideChannelInfo> {
  return invoke<ReflectSideChannelInfo>('reflect_get_side_channel', { id });
}
