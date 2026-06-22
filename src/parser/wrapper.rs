/// Privilege-escalation or execution wrappers that always require double-confirmation.
/// Includes shell interpreters (sh/bash/zsh/fish) because they can execute arbitrary code.
const WRAPPERS_HARD: &[&str] = &[
    "sudo", "doas", "su", "runuser", "nsenter", "unshare", "exec",
    "sh", "bash", "zsh", "fish",
];

/// Soft wrappers — only require double-confirmation when the segment contains
/// shell metacharacters (pipes, redirects, subshells, etc.).
const WRAPPERS_SOFT: &[&str] = &["env", "xargs", "nohup"];

pub fn is_hard_wrapper(cmd: &str) -> bool {
    WRAPPERS_HARD.contains(&cmd)
}

pub fn is_soft_wrapper(cmd: &str) -> bool {
    WRAPPERS_SOFT.contains(&cmd)
}

/// Returns true if the segment contains unquoted shell metacharacters.
/// Used to decide whether a soft wrapper (env/xargs/nohup) needs double-confirmation.
pub(crate) fn has_unquoted_metacharacter(segment: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = segment.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                chars.next();
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if in_single || in_double => {}
            '|' | ';' | '&' | '<' | '>' | '(' | ')' | '{' | '}' | '$' | '`' | '*' | '?' => {
                return true
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_wrapper_detected() {
        assert!(is_hard_wrapper("sudo"));
        assert!(is_hard_wrapper("doas"));
        assert!(is_hard_wrapper("exec"));
        assert!(is_hard_wrapper("bash"));
    }

    #[test]
    fn soft_wrapper_detected() {
        assert!(is_soft_wrapper("env"));
        assert!(is_soft_wrapper("xargs"));
        assert!(is_soft_wrapper("nohup"));
    }

    #[test]
    fn hard_wrapper_not_soft() {
        assert!(!is_hard_wrapper("env"));
        assert!(!is_soft_wrapper("sudo"));
        assert!(!is_soft_wrapper("bash"));
    }

    #[test]
    fn metacharacter_pipe() {
        assert!(has_unquoted_metacharacter("cat | grep"));
    }

    #[test]
    fn metacharacter_redirect() {
        assert!(has_unquoted_metacharacter("cat > file"));
    }

    #[test]
    fn metacharacter_quoted_is_safe() {
        assert!(!has_unquoted_metacharacter("echo 'a | b'"));
    }

    #[test]
    fn metacharacter_none() {
        assert!(!has_unquoted_metacharacter("env FOO=bar node app.js"));
    }
}
