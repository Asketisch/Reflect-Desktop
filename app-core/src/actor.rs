//! Actor 模型 —— 统一「谁做了什么」的语义层 (Phase 3 条目 8)。
//!
//! 几乎所有「谁做了什么」字段都用 `actor_type + actor_id`。
//! ReflectDesktop 不重构 核心 crate Task schema(避免改 核心 crate 镜像),而是把
//! `Actor` 作为语义层类型,编码进 `Task.metadata.actor`、`ActivityEvent.actor`
//! 与 Squad 的 `leaderActor` / `SquadMember.actor`。
//!
//! ## 约定
//!
//! - `actorId` 是稳定字符串:
//!   - `"user"` —— 本地人类用户(单用户桌面端假定)。
//!   - `"system"` —— 系统级动作(cron / autopilot / 崩溃恢复)。
//!   - `"<role>@<team>"` —— 团队成员(与 `reflect_task::TeamMemberSpec::agent_id` 同形)。
//!     例如 `"team-lead@rocket"` / `"architect@rocket"`。
//! - `ActorType` 是粗粒度枚举:`Human` / `Agent` / `System`,供 UI 用图标/颜色区分。
//! - `ActorKind` 是细粒度角色:`User` / `Lead` / `Member` / `System`,供 UI 徽标。
//!
//! ## 序列化
//!
//! 所有公开类型 `#[serde(rename_all = "camelCase")]`,与 TS 端 camelCase 对齐。

use serde::{Deserialize, Serialize};

/// 粗粒度 actor 类型。决定 UI 用人/机器人/齿轮图标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    /// 人类用户(本地桌面端用户)。
    Human,
    /// Agent(主 agent / subagent / team lead / team member)。
    Agent,
    /// 系统(cron / autopilot / 内部调度)。
    System,
}

impl ActorType {
    /// `"human"` / `"agent"` / `"system"`。
    pub fn as_str(self) -> &'static str {
        match self {
            ActorType::Human => "human",
            ActorType::Agent => "agent",
            ActorType::System => "system",
        }
    }
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 细粒度 actor 角色,供 UI 徽标/过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    /// 本地用户。
    User,
    /// 团队领导（`team-lead@<team>`）。
    Lead,
    /// Team member(非 lead 的 agent)。
    Member,
    /// 系统级动作。
    System,
}

impl ActorKind {
    /// `"user"` / `"lead"` / `"member"` / `"system"`。
    pub fn as_str(self) -> &'static str {
        match self {
            ActorKind::User => "user",
            ActorKind::Lead => "lead",
            ActorKind::Member => "member",
            ActorKind::System => "system",
        }
    }
}

impl std::fmt::Display for ActorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 统一 actor —— 「谁做了什么」的答案。
///
/// `actorId` 遵循 [`crate::actor`] 模块顶部约定的格式;`displayName`
/// 可选,UI 优先用 `displayName`,fallback 到 `actorId`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    /// 粗粒度类型。
    pub actor_type: ActorType,
    /// 稳定 id(`"user"` / `"system"` / `"<role>@<team>"`)。
    pub actor_id: String,
    /// 细粒度角色。
    pub kind: ActorKind,
    /// 人类可读名(可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// 所属 team 名(仅 agent 成员有,人类/系统为 None)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_name: Option<String>,
}

impl Actor {
    /// 构造本地用户 actor。
    pub fn user() -> Self {
        Self {
            actor_type: ActorType::Human,
            actor_id: "user".into(),
            kind: ActorKind::User,
            display_name: Some("You".into()),
            team_name: None,
        }
    }

    /// 构造系统 actor。
    pub fn system() -> Self {
        Self {
            actor_type: ActorType::System,
            actor_id: "system".into(),
            kind: ActorKind::System,
            display_name: Some("System".into()),
            team_name: None,
        }
    }

    /// 从 team 名 + role 字符串构造一个 agent actor。
    ///
    /// - `role` = `"team-lead"` → `ActorKind::Lead`,`actorId` = `"team-lead@<team>"`。
    /// - 其他 role → `ActorKind::Member`,`actorId` = `"<role>@<team>"`。
    pub fn agent(team: &str, role: &str) -> Self {
        let actor_id = format!("{role}@{team}");
        let kind = if role == "team-lead" {
            ActorKind::Lead
        } else {
            ActorKind::Member
        };
        Self {
            actor_type: ActorType::Agent,
            actor_id,
            kind,
            display_name: Some(role.to_string()),
            team_name: Some(team.to_string()),
        }
    }

    /// 用 `<role>@<team>` 字符串反解析出 actor(非 `@` 时退化为 system)。
    ///
    /// 与 [`reflect_task::team::parse_agent_id`] 的拆分逻辑对齐。
    pub fn from_agent_id(agent_id: &str) -> Self {
        match agent_id.split_once('@') {
            Some((role, team)) => Self::agent(team, role),
            None => {
                if agent_id == "user" {
                    Self::user()
                } else if agent_id == "system" {
                    Self::system()
                } else {
                    // 未知 id —— 保守归类为 system,避免误导。
                    Self {
                        actor_type: ActorType::System,
                        actor_id: agent_id.to_string(),
                        kind: ActorKind::System,
                        display_name: Some(agent_id.to_string()),
                        team_name: None,
                    }
                }
            }
        }
    }

    /// 把 actor 编码成可塞进 `Task.metadata.actor` 的 JSON value。
    ///
    /// 编码格式:`{"actor": { ...Actor serialized... }}`,外层包一层 `actor`
    /// key 避免与其他 metadata 字段冲突。
    pub fn encode_metadata(&self) -> serde_json::Value {
        serde_json::json!({ "actor": self })
    }

    /// 从 `Task.metadata`(或任意 JSON value)反解 actor。
    ///
    /// 接受两种形态:
    /// - `{ "actor": { ... } }` —— [`Self::encode_metadata`] 写入的形态。
    /// - `{ ...Actor fields... }` —— 直接是 actor object(宽容解析)。
    /// - 无 `actor` 字段且无法解析为 Actor → 返回 `None`。
    pub fn decode_metadata(metadata: &serde_json::Value) -> Option<Actor> {
        if let Some(actor_val) = metadata.get("actor") {
            serde_json::from_value(actor_val.clone()).ok()
        } else {
            // 尝试把整个 metadata 当作 actor object 解析。
            serde_json::from_value(metadata.clone()).ok()
        }
    }
}

impl Default for Actor {
    fn default() -> Self {
        Self::user()
    }
}

/// 把 `reflect_task::TeamMemberSpec` 转成 `Actor`。
///
/// `agent_id` 形如 `"<role>@<team>"`,经 [`Actor::from_agent_id`] 解析。
/// `name` 字段(人类可读)优先作为 `displayName`。
pub fn actor_from_team_member(member: &reflect_task::TeamMemberSpec) -> Actor {
    let mut actor = Actor::from_agent_id(&member.agent_id);
    if !member.name.is_empty() {
        actor.display_name = Some(member.name.clone());
    }
    actor
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_actor_serializes_camelcase() {
        let a = Actor::user();
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["actorType"], "human");
        assert_eq!(v["actorId"], "user");
        assert_eq!(v["kind"], "user");
        assert_eq!(v["displayName"], "You");
    }

    #[test]
    fn system_actor_kind_is_system() {
        let a = Actor::system();
        assert_eq!(a.actor_type, ActorType::System);
        assert_eq!(a.kind, ActorKind::System);
        assert_eq!(a.actor_id, "system");
    }

    #[test]
    fn lead_agent_detected_from_role() {
        let a = Actor::agent("rocket", "team-lead");
        assert_eq!(a.kind, ActorKind::Lead);
        assert_eq!(a.actor_id, "team-lead@rocket");
        assert_eq!(a.team_name.as_deref(), Some("rocket"));
    }

    #[test]
    fn member_agent_detected_from_role() {
        let a = Actor::agent("rocket", "architect");
        assert_eq!(a.kind, ActorKind::Member);
        assert_eq!(a.actor_id, "architect@rocket");
    }

    #[test]
    fn from_agent_id_roundtrip() {
        let a = Actor::from_agent_id("builder@rocket");
        assert_eq!(a.kind, ActorKind::Member);
        assert_eq!(a.team_name.as_deref(), Some("rocket"));
        let lead = Actor::from_agent_id("team-lead@alpha");
        assert_eq!(lead.kind, ActorKind::Lead);
    }

    #[test]
    fn from_agent_id_user_and_system() {
        assert_eq!(Actor::from_agent_id("user").kind, ActorKind::User);
        assert_eq!(Actor::from_agent_id("system").kind, ActorKind::System);
    }

    #[test]
    fn encode_decode_metadata_roundtrip() {
        let a = Actor::agent("rocket", "team-lead");
        let encoded = a.encode_metadata();
        let decoded = Actor::decode_metadata(&encoded).unwrap();
        assert_eq!(a, decoded);
    }

    #[test]
    fn decode_metadata_tolerates_bare_object() {
        // 宽容:整个 metadata 就是一个 actor object(没有外层 "actor" 包装)。
        let v = json!({
            "actorType": "agent",
            "actorId": "architect@rocket",
            "kind": "member"
        });
        let decoded = Actor::decode_metadata(&v).unwrap();
        assert_eq!(decoded.actor_id, "architect@rocket");
        assert_eq!(decoded.kind, ActorKind::Member);
    }

    #[test]
    fn decode_metadata_none_for_garbage() {
        let v = json!({ "foo": "bar" });
        assert!(Actor::decode_metadata(&v).is_none());
        assert!(Actor::decode_metadata(&json!(null)).is_none());
    }

    #[test]
    fn actor_from_team_member_uses_name_as_display() {
        let m = reflect_task::TeamMemberSpec {
            agent_id: "team-lead@rocket".into(),
            name: "Rocket Lead".into(),
            role: "team-lead".into(),
            model: None,
            system_prompt: String::new(),
            allowed_tools: vec![],
            color: None,
            joined_at: std::time::SystemTime::UNIX_EPOCH,
            session_id: None,
            subscriptions: vec![],
        };
        let a = actor_from_team_member(&m);
        assert_eq!(a.display_name.as_deref(), Some("Rocket Lead"));
        assert_eq!(a.kind, ActorKind::Lead);
    }

    #[test]
    fn actor_type_display_matches_serde() {
        assert_eq!(ActorType::Human.to_string(), "human");
        assert_eq!(ActorType::Agent.to_string(), "agent");
        assert_eq!(ActorType::System.to_string(), "system");
        assert_eq!(ActorKind::Lead.to_string(), "lead");
        assert_eq!(ActorKind::Member.to_string(), "member");
    }

    #[test]
    fn default_actor_is_user() {
        let a = Actor::default();
        assert_eq!(a.kind, ActorKind::User);
    }
}
