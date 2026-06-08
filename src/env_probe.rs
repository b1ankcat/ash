use crate::error::AshError;

pub struct EnvSummary {
    pub text: String,
}

pub fn collect(sys_info: bool, env_info: bool) -> Result<EnvSummary, AshError> {
    let mut parts = Vec::new();

    if sys_info {
        parts.push(format!("os={}", std::env::consts::OS));
        parts.push(format!("arch={}", std::env::consts::ARCH));
        // cwd is included as context but note it may reveal internal path structure
        if let Ok(cwd) = std::env::current_dir() {
            parts.push(format!("cwd={}", cwd.display()));
        }
    }

    if env_info {
        if let Ok(shell) = std::env::var("SHELL") {
            parts.push(format!("shell={shell}"));
        }
        if let Ok(term) = std::env::var("TERM") {
            parts.push(format!("term={term}"));
        }
        // PATH is intentionally not forwarded to avoid leaking internal infrastructure paths.
        let tools = ["git", "curl", "docker", "python3", "node", "cargo", "make"];
        let available: Vec<&str> = tools
            .iter()
            .filter(|t| which::which(t).is_ok())
            .copied()
            .collect();
        if !available.is_empty() {
            parts.push(format!("tools={}", available.join(",")));
        }
    }

    Ok(EnvSummary {
        text: parts.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_false_gives_empty() {
        let s = collect(false, false).unwrap();
        assert!(s.text.is_empty());
    }

    #[test]
    fn sys_info_includes_os_arch() {
        let s = collect(true, false).unwrap();
        assert!(s.text.contains("os="));
        assert!(s.text.contains("arch="));
    }

    #[test]
    fn env_info_does_not_include_path() {
        let s = collect(false, true).unwrap();
        // PATH must never be forwarded to the LLM
        assert!(!s.text.contains("PATH="));
    }
}
