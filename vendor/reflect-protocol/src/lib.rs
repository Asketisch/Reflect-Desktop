#![allow(clippy::derivable_impls)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::io_other_error)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::manual_div_ceil)]
//! reflect-protocol — Submission / Op / EventMsg / Item data structures.
//!
//! The protocol is the public contract between `reflect-core` and its clients
//! (TUI, exec, lib). It is async-friendly (mpsc), serializable (JSON), and
//! version-stable (v0 frozen; new variants are non-breaking additions).
//!
//! See `docs/protocol.md` for the full specification.

pub mod ask_user_input;
pub mod error;
pub mod event;
pub mod event_msg;
pub mod item;
pub mod op;
pub mod question;
pub mod recorder;
pub mod submission;

pub use error::ProtocolError;
pub use event::{EVENT_ID_NONE, Event};
pub use event_msg::{
    AbortReason, AgentMessage, AgentMessageDelta, ApprovalKind, ApprovalRequestEvent,
    AskUserInputEvent, CollabFinishedEvent, CollabMessageEvent, CollabStartedEvent,
    ConfigReloadedEvent, ContextCompactedEvent, ContextCompactedStrategy, ErrorEvent, EventMsg,
    LspServerFailedEvent, LspServerStartedEvent, McpServerFailedEvent, McpServerStartedEvent,
    McpToolInvokedEvent, McpTransportMirror, PermissionBubbleEvent, PermissionModeChangedEvent,
    PlanApprovedEvent, PlanReadyEvent, PlanRejectedEvent, PlanRequestEvent, RoutingEvent,
    RoutingEventKind, StreamErrorEvent, ThinkingDelta, TokenCountEvent, TokenUsage,
    ToolCallBeginEvent, ToolCallEndEvent, TriedCredential, TurnAbortedEvent, TurnCompleteEvent,
    TurnRewoundEvent, TurnStartedEvent, TurnStatus,
};
pub use item::{
    ApprovalPolicy, ContentBlock, PermissionMode, PlanId, ReasoningEffortMirror, ReviewDecision,
    RiskLevel, SandboxPolicy, SessionConfiguredEvent, ThreadId, ThreadSettingsOverrides, ToolError,
    ToolOutput, TurnId, UserInputItem, is_edit_tool_name,
};
pub use op::Op;
pub use question::{
    Answer, AskUserAnswer, AskUserQuestionEvent, MAX_HEADER_CHARS, MAX_OPTIONS, MAX_QUESTIONS,
    MIN_OPTIONS, Question, QuestionError, QuestionOption,
};
pub use recorder::{MessageRole, NullRecorder, RolloutRecord, RolloutRecorder, SessionInfo};
pub use submission::{Submission, W3cTraceContext};
