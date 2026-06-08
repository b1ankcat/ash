/// Result of parsing a shell command string.
pub struct ParseResult {
    pub commands: Vec<String>,
    pub has_unparseable: bool,
}

/// Commands that act as privilege-escalation or execution wrappers.
/// Presence of any wrapper triggers double-confirmation regardless of allow/deny lists.
const WRAPPERS: &[&str] = &[
    "sudo", "doas", "su", "runuser", "xargs", "nohup", "exec", "env", "sh", "bash", "zsh", "fish",
    "nsenter", "unshare",
];

pub fn is_wrapper(cmd: &str) -> bool {
    WRAPPERS.contains(&cmd)
}

pub fn parse(input: &str) -> ParseResult {
    let segments = split_on_operators(input);
    let mut commands = Vec::new();
    let mut has_unparseable = false;

    for seg in segments {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        match extract_primary(seg) {
            Some(cmd) => commands.push(cmd),
            None => has_unparseable = true,
        }
    }

    // Subshell syntax is never safe to execute without re-auditing the inner command.
    if contains_subshell(input) {
        has_unparseable = true;
    }

    ParseResult {
        commands,
        has_unparseable,
    }
}

/// Returns true if the input contains `$(` or backtick subshell syntax that the shell
/// would expand. Both are expanded inside double quotes but not single quotes.
fn contains_subshell(input: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                chars.next(); // skip escaped char
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if in_single => {} // single quotes suppress everything
            '`' => return true,
            '$' if chars.peek() == Some(&'(') => return true,
            _ => {}
        }
    }
    false
}

/// Split on `&&`, `||`, `;`, `|`, `&` — but only outside quotes.
/// Backslash escapes are honoured outside single quotes.
/// Both `&` (background) and `&&` (and) are treated as segment separators;
/// the semantic distinction is intentionally dropped in favour of auditing both sides.
fn split_on_operators(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                current.push(c);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(c);
            }
            _ if in_single || in_double => current.push(c),
            '&' => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                segments.push(current.clone());
                current.clear();
            }
            '|' => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                segments.push(current.clone());
                current.clear();
            }
            ';' => {
                segments.push(current.clone());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

/// Skip leading `VAR=val` assignments and return the first command token.
fn extract_primary(segment: &str) -> Option<String> {
    let words = shlex::split(segment)?;
    for word in words {
        if word.contains('=') && word.split('=').next().is_some_and(is_identifier) {
            continue;
        }
        return Some(word);
    }
    None
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmds(s: &str) -> Vec<String> {
        parse(s).commands
    }

    #[test]
    fn simple() {
        assert_eq!(cmds("ls -la"), vec!["ls"]);
    }

    #[test]
    fn pipeline() {
        assert_eq!(cmds("cat file | grep foo"), vec!["cat", "grep"]);
    }

    #[test]
    fn and_or_semicolon() {
        assert_eq!(
            cmds("apt-get update && rm -rf / && sudo time ls"),
            vec!["apt-get", "rm", "sudo"]
        );
        assert_eq!(cmds("a || b ; c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn background_operator() {
        assert_eq!(cmds("sleep 10 & echo done"), vec!["sleep", "echo"]);
    }

    #[test]
    fn quoted_separators_ignored() {
        assert_eq!(cmds("echo 'a && b'"), vec!["echo"]);
        assert_eq!(cmds(r#"echo "a | b""#), vec!["echo"]);
    }

    #[test]
    fn leading_env_assignments_skipped() {
        assert_eq!(cmds("NODE_ENV=prod FOO=bar node app.js"), vec!["node"]);
    }

    #[test]
    fn malformed_sets_unparseable() {
        let r = parse("echo 'unterminated");
        assert!(r.has_unparseable);
    }

    #[test]
    fn subshell_dollar_paren_sets_unparseable() {
        let r = parse("git log --format=$(date)");
        assert!(r.has_unparseable);
    }

    #[test]
    fn subshell_backtick_sets_unparseable() {
        let r = parse("echo `whoami`");
        assert!(r.has_unparseable);
    }

    #[test]
    fn quoted_subshell_is_still_flagged() {
        let r = parse(r#"echo "no $(subshell) here""#);
        // $( inside double quotes is still expanded by the shell — flag it for safety.
        assert!(r.has_unparseable);
    }

    #[test]
    fn backslash_escaped_quote() {
        // Escaped single quote should not toggle in_single
        assert_eq!(cmds(r"echo it\'s"), vec!["echo"]);
    }
}
