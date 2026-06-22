/// Shell builtin commands that have no on-disk binary. These pass the PATH check.
const BUILTINS: &[&str] = &[
    "cd", "source", ".", "export", "unset", "set", "alias", "unalias", "type", "read",
    "test", "[", "[[", "local", "umask", "ulimit", "wait", "jobs", "fg", "bg", "trap",
    "hash", "history", "shopt", "complete", "popd", "pushd", "dirs", "logout", "declare",
    "typeset", "echo", "printf", "pwd", "true", "false", "return", "break", "continue",
    "eval", "exec", "command", "builtin", "enable", "help", "let", "shift",
];

pub(crate) fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_cd_recognized() {
        assert!(is_builtin("cd"));
    }

    #[test]
    fn builtin_export_recognized() {
        assert!(is_builtin("export"));
    }

    #[test]
    fn non_builtin_ls_not_recognized() {
        assert!(!is_builtin("ls"));
    }
}
