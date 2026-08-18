/**
 * Agent 定义封装 —— Phase 1 第 3 项。
 *
 * 对应 `src-tauri/src/commands/agents.rs`(其内部封装
 * `reflect-agent/crates/abilities/reflect-agent-def`)。每个 agent 是一个带 YAML frontmatter 的
 * Markdown 文件,位于 `~/.reflect/agents/<name>.md`,与 TUI/CLI 共享。
 *
 * payload 形态与 核心 crate 的 `AgentDefinition` serde shape 一致(snake_case)。
 */
import { invoke } from '../bridge';

// ── 类型(对应 `reflect_agent_def::AgentDefinition`,snake_case) ──────────

export type ReflectMemoryScope = 'project' | 'user' | 'session';

export interface ReflectAgentDef {
  /** 稳定标识,同时作为文件名(`<name>.md`)。 */
  name: string;
  /** 人类可读的描述(必填)。 */
  description: string;
  /** 是否允许其他 agent 派生本 agent。 */
  spawnable: boolean;
  /** 标记为只读(无 write/edit 工具)。 */
  readonly: boolean;
  /** 工具白名单;为空表示允许全部内置工具。 */
  tools: string[];
  /** 工具黑名单。 */
  disallowed_tools: string[];
  /** `Some("inherit")` = 使用调用方模型;`Some(other)` = 覆盖;`null` = 跟随调用方。 */
  model: string | null;
  /** 单轮迭代次数硬上限。 */
  max_turns: number | null;
  /** 需要加载并注入的 memory 范围。 */
  memory: ReflectMemoryScope[];
  /** 预留给 v1 MCP 支持。 */
  mcp_collections: string[];
  /** Markdown 正文(即 system prompt)。 */
  system_prompt: string;
}

// ── Commands ───────────────────────────────────────────────────────────

/** 列出所有 agent 定义(按名称排序)。 */
export async function reflect_list_agent_defs(): Promise<ReflectAgentDef[]> {
  return invoke<ReflectAgentDef[]>('reflect_list_agent_defs');
}

/** 按名称读取单个 agent 定义,未找到时抛错。 */
export async function reflect_get_agent_def(name: string): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_get_agent_def', { name });
}

/**
 * 创建或更新 agent 定义。序列化到
 * `~/.reflect/agents/<name>.md` 并做往返校验。
 * `name` 与 `description` 为必填项。
 */
export async function reflect_save_agent_def(def: ReflectAgentDef): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_save_agent_def', { def });
}

/** 删除 agent 定义文件。返回 `true` 表示已删除,`false` 表示未找到。 */
export async function reflect_delete_agent_def(name: string): Promise<boolean> {
  return invoke<boolean>('reflect_delete_agent_def', { name });
}

/** 将 markdown 字符串解析/校验为 agent 定义(无副作用)。 */
export async function reflect_parse_agent_md(text: string): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_parse_agent_md', { text });
}
