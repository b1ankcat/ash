mod builtins;
mod patterns;

pub(crate) use builtins::is_builtin;
pub(crate) use patterns::{check_dangerous_pattern, check_pipe_to_shell};

use crate::config::Config;
use crate::parser;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    Safe,
    Mid,
    High,
}

#[derive(Debug)]
pub enum AuditSignal {
    NotInPath(String),
    DenyListHit(String),
    AllowListMiss(String),
    AllowDenyConflict(String),
    Wrapper(String),
    Unparseable,
    DangerousPattern(String),
}

impl fmt::Display for AuditSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInPath(cmd) => write!(f, "not in PATH: {cmd}"),
            Self::DenyListHit(cmd) => write!(f, "deny-list hit: {cmd}"),
            Self::AllowListMiss(cmd) => write!(f, "allow-list miss: {cmd}"),
            Self::AllowDenyConflict(cmd) => write!(f, "allow/deny conflict: {cmd}"),
            Self::Wrapper(cmd) => write!(f, "wrapper: {cmd}"),
            Self::Unparseable => write!(f, "unparseable syntax"),
            Self::DangerousPattern(desc) => write!(f, "dangerous pattern: {desc}"),
        }
    }
}

#[derive(Debug)]
pub struct AuditResult {
    pub commands: Vec<String>,
    pub should_reject: bool,
    pub need_double_confirm: bool,
    pub risk_level: RiskLevel,
    pub signals: Vec<AuditSignal>,
}

pub fn audit(parse: &parser::ParseResult, cfg: &Config) -> AuditResult {
    let mut should_reject = false;
    let mut need_double_confirm = false;
    let mut signals = Vec::new();

    if parse.has_unparseable {
        need_double_confirm = true;
        signals.push(AuditSignal::Unparseable);
    }

    // Cross-segment check: curl/wget piped to a shell interpreter.
    if let Some(desc) = check_pipe_to_shell(&parse.commands) {
        should_reject = true;
        signals.push(AuditSignal::DangerousPattern(desc.to_string()));
    }

    for (i, cmd) in parse.commands.iter().enumerate() {
        let in_allow = cfg.allow_list.is_empty() || cfg.allow_list.contains(cmd);
        let in_deny = cfg.deny_list.contains(cmd);
        let in_path = is_builtin(cmd) || which::which(cmd).is_ok();

        if !in_path {
            should_reject = true;
            signals.push(AuditSignal::NotInPath(cmd.clone()));
        }

        if in_deny {
            should_reject = true;
            signals.push(AuditSignal::DenyListHit(cmd.clone()));
        }

        if !cfg.allow_list.is_empty() && !in_allow {
            should_reject = true;
            signals.push(AuditSignal::AllowListMiss(cmd.clone()));
        }

        if in_deny && in_allow && !cfg.allow_list.is_empty() {
            signals.push(AuditSignal::AllowDenyConflict(cmd.clone()));
        }

        // Argument-level dangerous pattern check.
        if let Some(segment) = parse.segments.get(i)
            && let Some((seg_cmd, args)) = parser::extract_cmd_and_args(segment)
            && let Some(desc) = check_dangerous_pattern(&seg_cmd, &args)
        {
            should_reject = true;
            signals.push(AuditSignal::DangerousPattern(desc.to_string()));
        }

        // Wrapper check: hard wrappers always double-confirm; soft wrappers only
        // when the segment contains unquoted metacharacters.
        if parser::is_hard_wrapper(cmd) {
            need_double_confirm = true;
            signals.push(AuditSignal::Wrapper(cmd.clone()));
        } else if parser::is_soft_wrapper(cmd)
            && let Some(segment) = parse.segments.get(i)
            && parser::has_unquoted_metacharacter(segment)
        {
            need_double_confirm = true;
            signals.push(AuditSignal::Wrapper(cmd.clone()));
        }
    }

    let risk_level = if should_reject {
        RiskLevel::High
    } else if need_double_confirm {
        RiskLevel::Mid
    } else {
        RiskLevel::Safe
    };

    AuditResult {
        commands: parse.commands.clone(),
        should_reject,
        need_double_confirm,
        risk_level,
        signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_signal_display_not_in_path() {
        let s = AuditSignal::NotInPath("cd".into());
        assert_eq!(format!("{s}"), "not in PATH: cd");
    }

    #[test]
    fn audit_signal_display_dangerous() {
        let s = AuditSignal::DangerousPattern("rm -rf".into());
        assert_eq!(format!("{s}"), "dangerous pattern: rm -rf");
    }
}
