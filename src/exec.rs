use std::process::Command;

/// Shell allowlist — only these binaries are trusted as `-c` interpreters.
const ALLOWED_SHELLS: &[&str] = &[
    "/bin/sh",
    "/bin/bash",
    "/usr/bin/bash",
    "/usr/bin/zsh",
    "/bin/zsh",
];

/// Run a shell command, inheriting stdio, returning the child exit status.
///
/// `$SHELL` is validated against an allowlist and falls back to `/bin/sh`
/// to prevent environment-variable injection of arbitrary binaries.
/// `-i` is intentionally omitted to prevent alias/function shadowing of audited commands.
/// Note: the audit in `risk.rs` checks binary names only — it does not inspect arguments.
/// exec intentionally does not re-audit; the caller is responsible for running audit first.
pub fn run(command: &str) -> i32 {
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| ALLOWED_SHELLS.contains(&s.as_str()))
        .unwrap_or_else(|| "/bin/sh".into());
    Command::new(&shell)
        .arg("-c")
        .arg(command)
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1)
}

/// Print the shell-quoted command to stdout for manual inspection and re-use.
pub fn echo_for_edit(command: &str) {
    println!("{}", shell_quote(command));
}

/// Shell-quote a string using single quotes so it is safe to paste into a shell.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_for_edit_quotes_output() {
        let quoted = shell_quote("rm -rf /tmp/a b");
        assert_eq!(quoted, "'rm -rf /tmp/a b'");
    }

    #[test]
    fn echo_for_edit_escapes_single_quote() {
        let quoted = shell_quote("echo it's");
        assert_eq!(quoted, "'echo it'\\''s'");
    }
}
