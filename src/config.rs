use genai::adapter::AdapterKind;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub api_type: String,
    /// API key. Never printed — Config deliberately omits Debug.
    /// May be omitted from file if ASH_API_KEY env var is set.
    #[serde(default = "empty_string")]
    pub api_key: String,
    pub model_name: String,
    pub allow_list: Vec<String>,
    pub deny_list: Vec<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// LLM request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
    /// Tools to probe in PATH and include in LLM context.
    #[serde(default = "default_tools")]
    pub tools_to_probe: Vec<String>,
    #[serde(default = "default_true")]
    pub collect_sys_info: bool,
    #[serde(default = "default_true")]
    pub collect_env_info: bool,
}

fn empty_string() -> String {
    String::new()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    60
}

fn default_tools() -> Vec<String> {
    ["git", "curl", "docker", "python3", "node", "cargo", "make"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Config {
    pub fn adapter_kind(&self) -> Result<AdapterKind, crate::error::AshError> {
        match self.api_type.to_lowercase().as_str() {
            "openai" => Ok(AdapterKind::OpenAI),
            "anthropic" => Ok(AdapterKind::Anthropic),
            "gemini" => Ok(AdapterKind::Gemini),
            "ollama" => Ok(AdapterKind::Ollama),
            "groq" => Ok(AdapterKind::Groq),
            "cohere" => Ok(AdapterKind::Cohere),
            "deepseek" => Ok(AdapterKind::DeepSeek),
            "xai" => Ok(AdapterKind::Xai),
            other => Err(crate::error::AshError::InvalidConfig(format!(
                "unsupported api_type: {other}"
            ))),
        }
    }
}

pub fn load() -> Result<Config, crate::error::AshError> {
    let path = find_config()?;
    // Refuse symlinks — config must be a regular file to prevent TOCTOU and
    // symlink-based config injection.
    let meta = std::fs::symlink_metadata(&path).map_err(|e| {
        crate::error::AshError::InvalidConfig(format!("cannot stat config: {e}"))
    })?;
    if meta.file_type().is_symlink() {
        return Err(crate::error::AshError::SymlinkConfig(format!(
            "config file must not be a symlink: {}",
            path.display()
        )));
    }
    // Open the file once and read from the handle — avoids re-resolving the
    // path between the symlink check and the read (TOCTOU hardening).
    let mut file = std::fs::File::open(&path).map_err(|e| {
        crate::error::AshError::InvalidConfig(format!("cannot open config: {e}"))
    })?;
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut file, &mut raw).map_err(|e| {
        crate::error::AshError::InvalidConfig(format!("cannot read config: {e}"))
    })?;
    let cfg: Config =
        toml::from_str(&raw).map_err(|e| crate::error::AshError::InvalidConfig(e.to_string()))?;
    validate(cfg)
}

fn find_config() -> Result<PathBuf, crate::error::AshError> {
    let cwd = std::env::current_dir().ok().map(|d| d.join("config.toml"));
    if let Some(p) = cwd.filter(|p| p.exists()) {
        return Ok(p);
    }
    if let Some(home) = dirs::config_dir() {
        let p = home.join("ash/config.toml");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(crate::error::AshError::NoConfig)
}

fn validate(mut cfg: Config) -> Result<Config, crate::error::AshError> {
    // Allow the API key to be supplied via environment variable instead of the config file.
    if let Some(key) = std::env::var("ASH_API_KEY").ok().filter(|k| !k.is_empty()) {
        cfg.api_key = key;
    }
    let inv = |f: &str| crate::error::AshError::InvalidConfig(format!("{f} must not be empty"));
    for (val, name) in [
        (cfg.api_type.as_str(), "api_type"),
        (cfg.api_key.as_str(), "api_key"),
        (cfg.model_name.as_str(), "model_name"),
    ] {
        if val.is_empty() {
            return Err(inv(name));
        }
    }
    if cfg.request_timeout_secs == 0 {
        return Err(crate::error::AshError::InvalidConfig(
            "request_timeout_secs must be greater than 0".into(),
        ));
    }
    cfg.adapter_kind()?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AshError;

    fn parse(s: &str) -> Result<Config, AshError> {
        let cfg: Config = toml::from_str(s).map_err(|e| AshError::InvalidConfig(e.to_string()))?;
        validate(cfg)
    }

    fn base_toml() -> &'static str {
        r#"
api_type = "openai"
api_key = "sk-test"
model_name = "gpt-4"
allow_list = ["ls"]
deny_list = []
"#
    }

    #[test]
    fn valid_minimal() {
        assert!(parse(base_toml()).is_ok());
    }

    #[test]
    fn missing_required_field() {
        let s = r#"
api_type = "openai"
api_key = "sk-test"
allow_list = []
deny_list = []
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn empty_api_key_invalid() {
        let s = r#"
api_type = "openai"
api_key = ""
model_name = "gpt-4"
allow_list = []
deny_list = []
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn api_key_omitted_invalid_without_env() {
        // Without ASH_API_KEY env, omitted api_key must fail validation.
        let s = r#"
api_type = "openai"
model_name = "gpt-4"
allow_list = []
deny_list = []
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn unsupported_api_type() {
        let s = r#"
api_type = "foobar"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn optional_defaults() {
        let cfg = parse(base_toml()).unwrap();
        assert!(cfg.collect_sys_info);
        assert!(cfg.collect_env_info);
        assert!(cfg.base_url.is_none());
        assert_eq!(cfg.request_timeout_secs, 60);
        assert_eq!(cfg.tools_to_probe.len(), 7);
    }

    #[test]
    fn zero_timeout_invalid() {
        let s = r#"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
request_timeout_secs = 0
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn custom_tools_to_probe() {
        let s = r#"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
tools_to_probe = ["rg", "jq"]
"#;
        let cfg = parse(s).unwrap();
        assert_eq!(cfg.tools_to_probe, vec!["rg", "jq"]);
    }

    #[test]
    fn no_config_returns_err_c001() {
        let result = find_config_in(
            Some(std::path::PathBuf::from(
                "/nonexistent/__ash_test__/config.toml",
            )),
            None,
        );
        assert!(matches!(result, Err(AshError::NoConfig)));
    }

    // Extracted helper so tests never mutate process-global cwd.
    fn find_config_in(
        cwd_path: Option<PathBuf>,
        home_path: Option<PathBuf>,
    ) -> Result<PathBuf, AshError> {
        if let Some(p) = cwd_path.filter(|p| p.exists()) {
            return Ok(p);
        }
        if let Some(p) = home_path.filter(|p| p.exists()) {
            return Ok(p);
        }
        Err(AshError::NoConfig)
    }
}
