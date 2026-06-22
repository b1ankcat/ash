use crate::error::AshError;

pub struct EnvSummary {
    pub text: String,
}

pub fn collect(
    sys_info: bool,
    env_info: bool,
    tools_to_probe: &[String],
) -> Result<EnvSummary, AshError> {
    let mut parts = Vec::new();

    if sys_info {
        parts.push(format!("os={}", std::env::consts::OS));
        parts.push(format!("arch={}", std::env::consts::ARCH));
        // cwd is included as context but note it may reveal internal path structure
        let cwd = std::env::current_dir().map_err(|e| {
            AshError::EnvProbeError(format!("cannot get cwd: {e}"))
        })?;
        parts.push(format!("cwd={}", cwd.display()));
    }

    if env_info {
        if let Ok(shell) = std::env::var("SHELL") {
            parts.push(format!("shell={shell}"));
        }
        if let Ok(term) = std::env::var("TERM") {
            parts.push(format!("term={term}"));
        }
        // PATH is intentionally not forwarded to avoid leaking internal infrastructure paths.
        let available: Vec<&str> = tools_to_probe
            .iter()
            .filter(|t| which::which(t).is_ok())
            .map(|s| s.as_str())
            .collect();
        if !available.is_empty() {
            parts.push(format!("tools={}", available.join(",")));
        }
    }

    Ok(EnvSummary {
        text: parts.join("\n"),
    })
}
