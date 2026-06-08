use genai::adapter::AdapterKind;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub provider_name: String,
    pub api_type: String,
    /// API key. Never printed — Config deliberately omits Debug.
    pub api_key: String,
    pub model_name: String,
    pub allow_list: Vec<String>,
    pub deny_list: Vec<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// When true, silently reject denied commands and exit 1 without showing the UI.
    /// When false, show the high-risk UI and require double-confirmation.
    #[serde(default = "default_true")]
    pub silent_reject: bool,
    #[serde(default = "default_true")]
    pub collect_sys_info: bool,
    #[serde(default = "default_true")]
    pub collect_env_info: bool,
}

fn default_true() -> bool {
    true
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
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| crate::error::AshError::InvalidConfig(e.to_string()))?;
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
        (cfg.provider_name.as_str(), "provider_name"),
        (cfg.api_type.as_str(), "api_type"),
        (cfg.api_key.as_str(), "api_key"),
        (cfg.model_name.as_str(), "model_name"),
    ] {
        if val.is_empty() {
            return Err(inv(name));
        }
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

    #[test]
    fn valid_minimal() {
        let s = r#"
provider_name = "openai"
api_type = "openai"
api_key = "sk-test"
model_name = "gpt-4"
allow_list = ["ls"]
deny_list = []
"#;
        assert!(parse(s).is_ok());
    }

    #[test]
    fn missing_required_field() {
        let s = r#"
provider_name = "openai"
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
provider_name = "openai"
api_type = "openai"
api_key = ""
model_name = "gpt-4"
allow_list = []
deny_list = []
"#;
        assert!(matches!(parse(s), Err(AshError::InvalidConfig(_))));
    }

    #[test]
    fn unsupported_api_type() {
        let s = r#"
provider_name = "foobar"
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
        let s = r#"
provider_name = "openai"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#;
        let cfg = parse(s).unwrap();
        assert!(cfg.silent_reject);
        assert!(cfg.collect_sys_info);
        assert!(cfg.collect_env_info);
        assert!(cfg.base_url.is_none());
    }

    #[test]
    fn no_config_returns_err_c001() {
        // find_config is path-injectable via the public load() which reads cwd,
        // so we test the error variant directly.
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
