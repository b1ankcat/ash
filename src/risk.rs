use crate::config::Config;
use crate::parser;

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

    for cmd in &parse.commands {
        // Empty allow_list means open-world (all commands allowed); only deny_list applies.
        let in_allow = cfg.allow_list.is_empty() || cfg.allow_list.contains(cmd);
        let in_deny = cfg.deny_list.contains(cmd);
        let in_path = which::which(cmd).is_ok();

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

        // A command on both lists is a configuration conflict — deny wins (already rejected above).
        if in_deny && in_allow && !cfg.allow_list.is_empty() {
            signals.push(AuditSignal::AllowDenyConflict(cmd.clone()));
        }

        if parser::is_wrapper(cmd) {
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
    use crate::parser::ParseResult;

    fn cfg(allow: &[&str], deny: &[&str]) -> Config {
        Config {
            provider_name: "openai".into(),
            api_type: "openai".into(),
            api_key: "k".into(),
            model_name: "m".into(),
            allow_list: allow.iter().map(|s| s.to_string()).collect(),
            deny_list: deny.iter().map(|s| s.to_string()).collect(),
            base_url: None,
            silent_reject: true,
            collect_sys_info: true,
            collect_env_info: true,
        }
    }

    fn parse(cmds: &[&str], unparseable: bool) -> ParseResult {
        ParseResult {
            commands: cmds.iter().map(|s| s.to_string()).collect(),
            has_unparseable: unparseable,
        }
    }

    #[test]
    fn deny_list_rejects() {
        let r = audit(&parse(&["rm"], false), &cfg(&[], &["rm"]));
        assert!(r.should_reject);
    }

    #[test]
    fn allow_list_miss_rejects() {
        let r = audit(&parse(&["curl"], false), &cfg(&["ls", "git"], &[]));
        assert!(r.should_reject);
    }

    #[test]
    fn allow_deny_conflict_rejects() {
        // deny wins when a command appears in both lists
        let r = audit(&parse(&["git"], false), &cfg(&["git"], &["git"]));
        assert!(r.should_reject);
    }

    #[test]
    fn wrapper_double_confirm() {
        let r = audit(&parse(&["sudo"], false), &cfg(&[], &[]));
        assert!(r.need_double_confirm);
    }

    #[test]
    fn unparseable_double_confirm() {
        let r = audit(&parse(&[], true), &cfg(&[], &[]));
        assert!(r.need_double_confirm);
    }

    #[test]
    fn missing_from_path_rejects() {
        let r = audit(&parse(&["__ash_nonexistent_xyz__"], false), &cfg(&[], &[]));
        assert!(r.should_reject);
    }

    #[test]
    fn risk_level_safe() {
        let r = audit(&parse(&["ls"], false), &cfg(&[], &[]));
        // ls is in PATH on any normal system
        assert_eq!(r.risk_level, RiskLevel::Safe);
    }

    #[test]
    fn risk_level_mid() {
        let r = audit(&parse(&["sudo"], false), &cfg(&[], &[]));
        assert_eq!(r.risk_level, RiskLevel::Mid);
    }
}
