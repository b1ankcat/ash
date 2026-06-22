/// Detect curl/wget piped to a shell interpreter across pipeline segments.
pub(crate) fn check_pipe_to_shell(commands: &[String]) -> Option<&'static str> {
    let has_fetch = commands.iter().any(|c| c == "curl" || c == "wget");
    let has_shell = commands.iter().any(|c| {
        matches!(
            c.as_str(),
            "sh" | "bash" | "zsh" | "fish" | "python" | "python3" | "perl" | "ruby" | "node"
        )
    });
    if has_fetch && has_shell {
        Some("curl/wget piped to shell interpreter")
    } else {
        None
    }
}

/// Detect known dangerous argument patterns per command.
pub(crate) fn check_dangerous_pattern(cmd: &str, args: &[String]) -> Option<&'static str> {
    match cmd {
        "rm" => {
            if args.is_empty() {
                return Some("rm without path argument");
            }
            let has_r = args.iter().any(|a| {
                a == "-r" || a == "-R" || a == "-rf" || a == "-fr" || a == "-rF" || a.contains("--recursive")
            });
            let has_f = args.iter().any(|a| {
                a == "-f" || a == "-rf" || a == "-fr" || a.contains("--force")
            });
            if has_r && has_f {
                return Some("rm -rf: recursive force delete");
            }
            if has_r {
                return Some("rm -r: recursive delete");
            }
            if has_f && args.iter().any(|a| a == "/" || a == "/*") {
                return Some("rm -f /: force delete root");
            }
        }
        "git" => {
            if args.first().is_some_and(|a| a == "push")
                && args.iter().any(|a| a == "-f" || a == "--force" || a == "--force-with-lease")
            {
                return Some("git push --force: rewrites remote history");
            }
        }
        "chmod" => {
            if args.first().is_some_and(|a| a == "777" || a == "a+rwx" || a == "000") {
                return Some("chmod with extreme permission bits");
            }
        }
        "chown" => {
            if args.iter().any(|a| a == "-R" || a.contains("--recursive")) {
                return Some("chown -R: recursive ownership change");
            }
        }
        "dd" => {
            if args.iter().any(|a| a.starts_with("of=/dev/")) {
                return Some("dd to device: disk overwrite");
            }
        }
        "mkfs" | "mke2fs" | "mkfs.ext4" | "mkfs.xfs" | "mkfs.btrfs" | "mkfs.f2fs" => {
            return Some("mkfs: filesystem format");
        }
        "shutdown" | "reboot" | "halt" | "poweroff" | "init" => {
            return Some("system power control");
        }
        "killall" => return Some("killall: kill by name"),
        "pkill" => return Some("pkill: kill by pattern"),
        "iptables" | "nft" => return Some("firewall rule modification"),
        "sysctl" if args.iter().any(|a| a == "-w") => {
            return Some("sysctl -w: kernel parameter write");
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_rm_rf() {
        assert_eq!(
            check_dangerous_pattern("rm", &["-rf".into(), "/tmp".into()]),
            Some("rm -rf: recursive force delete")
        );
    }

    #[test]
    fn dangerous_rm_r() {
        assert_eq!(
            check_dangerous_pattern("rm", &["-r".into(), "/tmp".into()]),
            Some("rm -r: recursive delete")
        );
    }

    #[test]
    fn dangerous_rm_no_args() {
        assert_eq!(
            check_dangerous_pattern("rm", &[]),
            Some("rm without path argument")
        );
    }

    #[test]
    fn dangerous_rm_f_root() {
        assert_eq!(
            check_dangerous_pattern("rm", &["-f".into(), "/".into()]),
            Some("rm -f /: force delete root")
        );
    }

    #[test]
    fn safe_rm_file() {
        assert_eq!(check_dangerous_pattern("rm", &["file.txt".into()]), None);
    }

    #[test]
    fn dangerous_git_push_force() {
        assert_eq!(
            check_dangerous_pattern("git", &["push".into(), "--force".into()]),
            Some("git push --force: rewrites remote history")
        );
    }

    #[test]
    fn safe_git_push() {
        assert_eq!(
            check_dangerous_pattern("git", &["push".into(), "origin".into()]),
            None
        );
    }

    #[test]
    fn dangerous_chmod_777() {
        assert_eq!(
            check_dangerous_pattern("chmod", &["777".into(), "/file".into()]),
            Some("chmod with extreme permission bits")
        );
    }

    #[test]
    fn dangerous_dd_to_device() {
        assert_eq!(
            check_dangerous_pattern("dd", &["of=/dev/sda".into()]),
            Some("dd to device: disk overwrite")
        );
    }

    #[test]
    fn dangerous_mkfs() {
        assert_eq!(
            check_dangerous_pattern("mkfs", &["/dev/sda1".into()]),
            Some("mkfs: filesystem format")
        );
    }

    #[test]
    fn pipe_to_shell_detected() {
        let cmds = vec!["curl".to_string(), "sh".to_string()];
        assert_eq!(
            check_pipe_to_shell(&cmds),
            Some("curl/wget piped to shell interpreter")
        );
    }

    #[test]
    fn pipe_to_python_detected() {
        let cmds = vec!["wget".to_string(), "python3".to_string()];
        assert_eq!(
            check_pipe_to_shell(&cmds),
            Some("curl/wget piped to shell interpreter")
        );
    }

    #[test]
    fn no_pipe_to_shell() {
        let cmds = vec!["ls".to_string(), "grep".to_string()];
        assert_eq!(check_pipe_to_shell(&cmds), None);
    }
}
