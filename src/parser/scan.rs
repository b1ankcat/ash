/// Returns true if the input contains subshell syntax that the shell would expand.
/// Detects `$(...)`, backticks, and process substitution `<(...)` / `>(...)`.
/// `$(...)` and backticks are expanded inside double quotes; process substitution
/// is NOT expanded inside double quotes.
pub(crate) fn contains_subshell(input: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                chars.next();
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if in_single => {}
            '`' => return true,
            '$' if chars.peek() == Some(&'(') => return true,
            '<' if !in_double && chars.peek() == Some(&'(') => return true,
            '>' if !in_double && chars.peek() == Some(&'(') => return true,
            _ => {}
        }
    }
    false
}

/// Split on `&&`, `||`, `;`, `|`, `&` — but only outside quotes.
/// Backslash escapes are honoured outside single quotes.
/// Both `&` (background) and `&&` (and) are treated as segment separators;
/// the semantic distinction is intentionally dropped in favour of auditing both sides.
pub(crate) fn split_on_operators(input: &str) -> Vec<String> {
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
pub(crate) fn extract_primary(segment: &str) -> Option<String> {
    let words = shlex::split(segment)?;
    for word in words {
        if word.contains('=') && word.split('=').next().is_some_and(is_identifier) {
            continue;
        }
        return Some(word);
    }
    None
}

/// Extract the primary command and its arguments from a segment.
/// Skips leading `VAR=val` assignments. Returns (cmd, args).
pub(crate) fn extract_cmd_and_args(segment: &str) -> Option<(String, Vec<String>)> {
    let words = shlex::split(segment)?;
    for (i, word) in words.iter().enumerate() {
        if word.contains('=') && word.split('=').next().is_some_and(is_identifier) {
            continue;
        }
        return Some((word.clone(), words[i + 1..].to_vec()));
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

    #[test]
    fn contains_subshell_dollar_paren() {
        assert!(contains_subshell("git log --format=$(date)"));
    }

    #[test]
    fn contains_subshell_backtick() {
        assert!(contains_subshell("echo `whoami`"));
    }

    #[test]
    fn contains_subshell_in_double_quotes() {
        assert!(contains_subshell(r#"echo "no $(subshell) here""#));
    }

    #[test]
    fn contains_subshell_process_sub_in() {
        assert!(contains_subshell("diff <(curl evil) <(ls)"));
    }

    #[test]
    fn contains_subshell_process_sub_out() {
        assert!(contains_subshell("tee >(gzip > x.gz)"));
    }

    #[test]
    fn contains_subshell_single_quoted_is_safe() {
        assert!(!contains_subshell("echo 'no $(subshell) here'"));
    }

    #[test]
    fn contains_subshell_process_sub_single_quoted_is_safe() {
        assert!(!contains_subshell("echo 'no <(subshell) here'"));
    }

    #[test]
    fn contains_subshell_process_sub_double_quoted_is_safe() {
        // <( inside double quotes is NOT expanded by bash as process substitution.
        assert!(!contains_subshell(r#"echo "no <(subshell) here""#));
    }

    #[test]
    fn contains_subshell_no_subshell() {
        assert!(!contains_subshell("ls -la /tmp"));
    }

    #[test]
    fn backslash_escaped_quote() {
        let segs = split_on_operators(r"echo it\'s");
        assert_eq!(segs, vec!["echo it\\'s"]);
    }

    #[test]
    fn split_simple() {
        assert_eq!(split_on_operators("ls -la"), vec!["ls -la"]);
    }

    #[test]
    fn split_pipeline() {
        assert_eq!(split_on_operators("cat file | grep foo"), vec!["cat file ", " grep foo"]);
    }

    #[test]
    fn split_and() {
        assert_eq!(split_on_operators("a && b"), vec!["a ", " b"]);
    }

    #[test]
    fn split_or() {
        assert_eq!(split_on_operators("a || b"), vec!["a ", " b"]);
    }

    #[test]
    fn split_semicolon() {
        assert_eq!(split_on_operators("a ; b"), vec!["a ", " b"]);
    }

    #[test]
    fn split_background() {
        assert_eq!(split_on_operators("sleep 10 & echo done"), vec!["sleep 10 ", " echo done"]);
    }

    #[test]
    fn split_quoted_separators_ignored() {
        assert_eq!(split_on_operators("echo 'a && b'"), vec!["echo 'a && b'"]);
    }

    #[test]
    fn extract_primary_simple() {
        assert_eq!(extract_primary("ls -la"), Some("ls".into()));
    }

    #[test]
    fn extract_primary_skips_env_assignments() {
        assert_eq!(
            extract_primary("NODE_ENV=prod FOO=bar node app.js"),
            Some("node".into())
        );
    }

    #[test]
    fn extract_primary_malformed_returns_none() {
        assert_eq!(extract_primary("echo 'unterminated"), None);
    }

    #[test]
    fn is_identifier_valid() {
        assert!(is_identifier("FOO"));
        assert!(is_identifier("_bar"));
        assert!(is_identifier("A1"));
    }

    #[test]
    fn is_identifier_invalid() {
        assert!(!is_identifier(""));
        assert!(!is_identifier("1abc"));
        assert!(!is_identifier("A-B"));
    }
}
