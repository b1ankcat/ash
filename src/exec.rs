use crate::error::AshError;
use std::path::PathBuf;
use std::process::Command;

/// Shell allowlist — only these binaries are trusted as `-c` interpreters.
/// Compared against the canonicalized path to prevent symlink bypass.
const ALLOWED_SHELLS: &[&str] = &[
    "/bin/sh",
    "/bin/bash",
    "/usr/bin/bash",
    "/usr/bin/zsh",
    "/bin/zsh",
];

/// Resolve $SHELL to a canonicalized path and validate against the allowlist.
/// No fallback — if $SHELL is unset, uncanonicalizable, or not allowlisted,
/// return an error. The caller must surface it, never silently substitute.
fn resolve_shell() -> Result<PathBuf, AshError> {
    let shell = std::env::var("SHELL")
        .map_err(|_| AshError::ShellNotAllowlisted("SHELL environment variable not set".into()))?;
    let canon = std::fs::canonicalize(&shell).map_err(|e| {
        AshError::ShellNotAllowlisted(format!("cannot canonicalize SHELL={shell}: {e}"))
    })?;
    let canon_str = canon.to_str().ok_or_else(|| {
        AshError::ShellNotAllowlisted(format!(
            "canonicalized SHELL path is not valid UTF-8: {}",
            canon.display()
        ))
    })?;
    if !ALLOWED_SHELLS.contains(&canon_str) {
        return Err(AshError::ShellNotAllowlisted(format!(
            "SHELL {shell} (canonicalized: {canon_str}) is not in the allowlist"
        )));
    }
    Ok(canon)
}

/// Run a shell command, inheriting stdio, returning the child exit status.
///
/// `$SHELL` is canonicalized and validated against an allowlist.
/// `-i` is intentionally omitted to prevent alias/function shadowing of audited commands.
/// exec intentionally does not re-audit; the caller is responsible for running audit first.
pub fn run(command: &str) -> Result<i32, AshError> {
    let shell = resolve_shell()?;
    let status = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .status()
        .map_err(|e| AshError::ExecError(format!("cannot execute shell: {e}")))?;
    status
        .code()
        .ok_or_else(|| AshError::ExecError("process terminated by signal".into()))
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
