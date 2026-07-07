/**
 * M1.2 占位协议 TS 类型 — 由 dump_schema 产出 JSON Schema 后,
 * `npx json2ts /tmp/reflect-schema.json -o src/types/protocol.ts` 生成。
 * 当前为手工精简版,只导出 `ReflectEvent`(IPC 载荷顶层 type)
 * 与 `ReflectSubmissionType`(用于构造 Submission 时声明 op 类别)。
 *
 * 完整 schema 版本见 `crates/reflect-protocol/examples/dump_schema.rs`,
 * 由 M2.x 升级到 schemars derive 后可 json2ts 一次生成。
 */

export type ReflectEventType =
  | 'session_configured'
  | 'turn_started'
  | 'turn_complete'
  | 'turn_aborted'
  | 'turn_rewound'
  | 'shutdown_complete'
  | 'agent_message'
  | 'agent_message_delta'
  | 'thinking_delta'
  | 'token_count'
  | 'tool_call_begin'
  | 'tool_call_end'
  | 'approval_request'
  | 'ask_user_question'
  | 'ask_user_input'
  | 'permission_bubble'
  | 'context_compacted'
  | 'error'
  | 'stream_error'
  | 'routing'
  | 'config_reloaded'
  | 'collab_started'
  | 'collab_message'
  | 'collab_finished'
  | 'mcp_server_started'
  | 'mcp_server_failed'
  | 'mcp_tool_invoked'
  | 'lsp_server_started'
  | 'lsp_server_failed'
  | 'plan_request'
  | 'plan_ready'
  | 'plan_approved'
  | 'plan_rejected'
  | 'permission_mode_changed';

export type ReflectSubmissionOpType =
  | 'user_input'
  | 'compact'
  | 'interrupt'
  | 'rewind'
  | 'shutdown'
  | 'tool_approval'
  | 'hook_approval'
  | 'enter_plan_mode'
  | 'exit_plan_mode'
  | 'plan_approval'
  | 'set_effort'
  | 'ask_user_question_response'
  | 'ask_user_input_response'
  | 'set_permission_mode'
  | 'cycle_permission_mode';

/**
 * Tauri event `reflect_event` 的 payload —— 直接以 `reflect_protocol::Event`
 * 序列化形态 (`{ id: string, msg: { type: ReflectEventType, ... } }`)。
 * `msg` 内的具体结构取决于 type,M1.4 起 reducer 才展开
 * (见 docs/todo/21-gui/04-chat-render.md)。
 */
export interface ReflectEvent {
  id: string;
  msg: {
    type: ReflectEventType;
    [k: string]: unknown;
  };
}

/**
 * 向后端 invoke `reflect_submit` 的 submission 形态 —— Rust 侧
 * `reflect_protocol::Submission` 的 TS 镜像。id 自动生成,前端通常
 * 只构造 op 字段。
 */
export interface ReflectSubmission {
  id: string;
  op:
    | { type: 'user_input'; items: ReflectUserInputItem[]; thread_settings?: Record<string, unknown> }
    | { type: 'interrupt' }
    | { type: 'shutdown' }
    | { type: 'compact' }
    | { type: 'rewind'; to_turn_id?: string | null }
    | { type: 'tool_approval'; id: string; decision: 'approve' | 'deny' | 'abort' }
    | { type: 'hook_approval'; id: string; decision: 'approve' | 'deny' | 'abort' }
    | { type: 'enter_plan_mode'; task: string }
    | { type: 'exit_plan_mode' }
    | { type: 'plan_approval'; id: string; decision: 'approve' | 'deny' }
    | { type: 'set_effort'; level: string }
    | { type: 'ask_user_question_response'; id: string; answers: unknown }
    | { type: 'ask_user_input_response'; id: string; text: string }
    | { type: 'set_permission_mode'; mode: string }
    | { type: 'cycle_permission_mode' };
  client_user_message_id?: string;
}

export interface ReflectUserInputItem {
  type: 'text' | 'image' | 'local_image' | 'skill' | 'question_answer';
  text?: string;
  data?: string; // base64
  mime_type?: string;
  path?: string;
  name?: string;        // skill name
  args?: unknown;
  request_id?: string;  // question_answer
  answers?: unknown;    // question_answer
}
