//! `matcher` —— glob / shell 规则匹配(P2 `permission-rules`)。
//!
//! 在精确 `tool` 匹配之上扩展:
//! - `tool_glob`:工具名 glob(如 `Web*`)
//! - `shell_pattern`:Bash 命令 glob(仅 `Bash`/`bash` 工具生效)

use globset::{Glob, GlobMatcher};

use crate::rules::{PermissionAction, PermissionRule, RuleMatch};

/// 编译后的 shell 规则 —— 避免每次 evaluate 重复解析 glob。
#[derive(Debug, Clone)]
pub struct CompiledShellRule {
    pub tool: String,
    pub pattern: GlobMatcher,
    pub action: PermissionAction,
}

impl CompiledShellRule {
    /// 从 `PermissionRule` 编译 shell 规则;无 `shell_pattern` 时返回 `None`。
    pub fn compile(rule: &PermissionRule) -> Result<Option<Self>, globset::Error> {
        let Some(pat) = rule.shell_pattern.as_deref() else {
            return Ok(None);
        };
        let glob = Glob::new(pat)?;
        Ok(Some(Self {
            tool: rule.tool.clone(),
            pattern: glob.compile_matcher(),
            action: rule.action,
        }))
    }
}

/// 带上下文的规则评估 —— 支持 tool glob 与 Bash shell 模式。
///
/// 匹配顺序与 v1.x 一致:按 rules 数组顺序,**第一条**命中即返回。
/// shell 规则仅在 `bash_command` 有值且工具名与规则 `tool` 一致(忽略大小写)时参与。
pub fn evaluate_with_context(
    rules: &[PermissionRule],
    tool_name: &str,
    bash_command: Option<&str>,
) -> RuleMatch {
    for rule in rules {
        if let Some(m) = match_rule(rule, tool_name, bash_command) {
            return m;
        }
    }
    RuleMatch::NoMatch
}

fn match_rule(
    rule: &PermissionRule,
    tool_name: &str,
    bash_command: Option<&str>,
) -> Option<RuleMatch> {
    // shell 规则优先:需要命令上下文。
    if let Some(pat) = rule.shell_pattern.as_deref() {
        let cmd = bash_command?;
        if !tool_names_equal(&rule.tool, tool_name) {
            return None;
        }
        let glob = Glob::new(pat).ok()?;
        if glob.compile_matcher().is_match(cmd) {
            return Some(action_to_match(rule.action));
        }
        return None;
    }

    // tool_glob 规则。
    if let Some(glob_str) = rule.tool_glob.as_deref() {
        let glob = Glob::new(glob_str).ok()?;
        if glob.compile_matcher().is_match(tool_name) {
            return Some(action_to_match(rule.action));
        }
        return None;
    }

    // 精确 tool 匹配(v1.x 行为)。
    if rule.tool == tool_name {
        return Some(action_to_match(rule.action));
    }
    None
}

fn tool_names_equal(expected: &str, actual: &str) -> bool {
    expected.eq_ignore_ascii_case(actual)
}

fn action_to_match(action: PermissionAction) -> RuleMatch {
    match action {
        PermissionAction::Allow => RuleMatch::Allow,
        PermissionAction::Deny => RuleMatch::Deny,
        PermissionAction::Ask => RuleMatch::Ask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::PermissionRule;

    fn rule(tool: &str, action: PermissionAction) -> PermissionRule {
        PermissionRule {
            tool: tool.into(),
            action,
            tool_glob: None,
            shell_pattern: None,
        }
    }

    #[test]
    fn tool_glob_matches_prefix() {
        let rules = vec![PermissionRule {
            tool: String::new(),
            action: PermissionAction::Allow,
            tool_glob: Some("Web*".into()),
            shell_pattern: None,
        }];
        assert_eq!(
            evaluate_with_context(&rules, "WebFetch", None),
            RuleMatch::Allow
        );
        assert_eq!(
            evaluate_with_context(&rules, "Read", None),
            RuleMatch::NoMatch
        );
    }

    #[test]
    fn shell_pattern_matches_bash_command() {
        let rules = vec![PermissionRule {
            tool: "Bash".into(),
            action: PermissionAction::Allow,
            tool_glob: None,
            shell_pattern: Some("git *".into()),
        }];
        assert_eq!(
            evaluate_with_context(&rules, "Bash", Some("git status")),
            RuleMatch::Allow
        );
        assert_eq!(
            evaluate_with_context(&rules, "Bash", Some("rm -rf /")),
            RuleMatch::NoMatch
        );
        // 非 Bash 工具不触发 shell 规则。
        assert_eq!(
            evaluate_with_context(&rules, "Write", Some("git status")),
            RuleMatch::NoMatch
        );
    }

    #[test]
    fn exact_tool_still_works() {
        let rules = vec![rule("Read", PermissionAction::Deny)];
        assert_eq!(evaluate_with_context(&rules, "Read", None), RuleMatch::Deny);
    }

    #[test]
    fn first_match_wins_across_rule_kinds() {
        let rules = vec![
            rule("Bash", PermissionAction::Deny),
            PermissionRule {
                tool: "Bash".into(),
                action: PermissionAction::Allow,
                tool_glob: None,
                shell_pattern: Some("git *".into()),
            },
        ];
        assert_eq!(
            evaluate_with_context(&rules, "Bash", Some("git pull")),
            RuleMatch::Deny,
            "精确规则在前,先于 shell 规则"
        );
    }
}
