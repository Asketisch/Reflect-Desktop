/**
 * Agent definition wrappers — Phase 1 item 3.
 *
 * Mirrors `src-tauri/src/commands/agents.rs` (which wraps
 * `vendor/reflect-agent-def`). Each agent is a Markdown file with YAML
 * frontmatter at `~/.reflect/agents/<name>.md`, shared with the TUI/CLI.
 *
 * Payloads mirror the vendor `AgentDefinition` serde shape (snake_case).
 */
import { invoke } from '../bridge';

// ── Types (mirror `reflect_agent_def::AgentDefinition`, snake_case) ────

export type ReflectMemoryScope = 'project' | 'user' | 'session';

export interface ReflectAgentDef {
  /** Stable identifier; also the filename (`<name>.md`). */
  name: string;
  /** Human-readable description (required). */
  description: string;
  /** Whether other agents may spawn this one. */
  spawnable: boolean;
  /** Mark as read-only (no write/edit tools). */
  readonly: boolean;
  /** Tool whitelist; empty = all builtins. */
  tools: string[];
  /** Tool denylist. */
  disallowed_tools: string[];
  /** `Some("inherit")` = use caller model; `Some(other)` = override; `null` = caller. */
  model: string | null;
  /** Hard cap on iterations per turn. */
  max_turns: number | null;
  /** Cap on tool-result character count per call. */
  max_result_chars: number | null;
  /** Memory scopes to load and inject. */
  memory: ReflectMemoryScope[];
  /** Reserved for v1 MCP support. */
  mcp_collections: string[];
  /** Markdown body (the system prompt). */
  system_prompt: string;
}

// ── Commands ───────────────────────────────────────────────────────────

/** List all agent definitions (sorted by name). */
export async function reflect_list_agent_defs(): Promise<ReflectAgentDef[]> {
  return invoke<ReflectAgentDef[]>('reflect_list_agent_defs');
}

/** Read a single agent definition by name. Throws on not-found. */
export async function reflect_get_agent_def(name: string): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_get_agent_def', { name });
}

/**
 * Create or update an agent definition. Serializes to
 * `~/.reflect/agents/<name>.md` and round-trip validates. `name` and
 * `description` are required.
 */
export async function reflect_save_agent_def(def: ReflectAgentDef): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_save_agent_def', { def });
}

/** Delete an agent definition file. Returns `true` if deleted, `false` if not found. */
export async function reflect_delete_agent_def(name: string): Promise<boolean> {
  return invoke<boolean>('reflect_delete_agent_def', { name });
}

/** Parse / validate a markdown string as an agent definition (no side effects). */
export async function reflect_parse_agent_md(text: string): Promise<ReflectAgentDef> {
  return invoke<ReflectAgentDef>('reflect_parse_agent_md', { text });
}
