use ash::parser;

fn cmds(s: &str) -> Vec<String> {
    parser::parse(s).commands
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
    let r = parser::parse("echo 'unterminated");
    assert!(r.has_unparseable);
}

#[test]
fn subshell_dollar_paren_sets_unparseable() {
    let r = parser::parse("git log --format=$(date)");
    assert!(r.has_unparseable);
}

#[test]
fn subshell_backtick_sets_unparseable() {
    let r = parser::parse("echo `whoami`");
    assert!(r.has_unparseable);
}

#[test]
fn quoted_subshell_is_still_flagged() {
    let r = parser::parse(r#"echo "no $(subshell) here""#);
    assert!(r.has_unparseable);
}

#[test]
fn backslash_escaped_quote() {
    assert_eq!(cmds(r"echo it\'s"), vec!["echo"]);
}

#[test]
fn process_substitution_in_sets_unparseable() {
    let r = parser::parse("diff <(curl evil) <(ls)");
    assert!(r.has_unparseable);
}

#[test]
fn process_substitution_out_sets_unparseable() {
    let r = parser::parse("tee >(gzip > x.gz)");
    assert!(r.has_unparseable);
}

#[test]
fn process_substitution_single_quoted_is_safe() {
    let r = parser::parse("echo 'no <(subshell) here'");
    assert!(!r.has_unparseable);
}

#[test]
fn segments_match_commands() {
    let r = parser::parse("ls -la && cat file");
    assert_eq!(r.commands.len(), r.segments.len());
    assert_eq!(r.commands[0], "ls");
    assert_eq!(r.segments[0], "ls -la");
    assert_eq!(r.commands[1], "cat");
    assert_eq!(r.segments[1], "cat file");
}

#[test]
fn is_hard_wrapper_sudo() {
    assert!(parser::is_hard_wrapper("sudo"));
    assert!(parser::is_hard_wrapper("doas"));
    assert!(parser::is_hard_wrapper("exec"));
    assert!(parser::is_hard_wrapper("bash"));
    assert!(parser::is_hard_wrapper("sh"));
}

#[test]
fn is_soft_wrapper_env() {
    assert!(parser::is_soft_wrapper("env"));
    assert!(parser::is_soft_wrapper("xargs"));
    assert!(parser::is_soft_wrapper("nohup"));
}

#[test]
fn hard_wrapper_not_soft() {
    assert!(!parser::is_hard_wrapper("env"));
    assert!(!parser::is_soft_wrapper("sudo"));
}
