/**
 * Agent 状态 / config TOML / 工具注册表 封装。
 *
 * `ReflectAgentStatus` 驱动状态徽标与降级模式 CTA。
 * TOML 命令封装 `~/.reflect/config.toml` 的原始读写 —— 后端在写入磁盘前
 * 仍会进行合法性校验与解析。
 */
import { invoke } from '../bridge';

/** agent 状态快照 —— 前端状态徽标 + 降级引导用。 */
export interface ReflectAgentStatus {
  ready: boolean;
  has_model: boolean;
  model: string;
  workspace: string;
  degraded_reason: string | null;
}

/** 单个工具的 name + description。 */
export interface ReflectToolInfo {
  name: string;
  description: string;
}

/** 返回 agent 状态(ready / has_model / model / workspace / degraded_reason)。 */
export async function reflect_agent_status(): Promise<ReflectAgentStatus> {
  return invoke<ReflectAgentStatus>('reflect_agent_status');
}

/** 读取 ~/.reflect/config.toml 的 TOML 字符串。Settings 页加载用。 */
export async function reflect_get_config(): Promise<string> {
  return invoke<string>('reflect_get_config');
}

/** 写回 ~/.reflect/config.toml(写盘前校验合法性)。 */
export async function reflect_save_config(toml: string): Promise<void> {
  return invoke<void>('reflect_save_config', { toml });
}

/** 列出当前 ToolRegistry 中所有工具(name + description)。 */
export async function reflect_list_tools(): Promise<ReflectToolInfo[]> {
  return invoke<ReflectToolInfo[]>('reflect_list_tools');
}
