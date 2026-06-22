use ash::config::Config;
use ash::error::AshError;

fn make_config(api_type: &str) -> Result<Config, AshError> {
    let toml_str = format!(
        r#"
api_type = "{}"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#,
        api_type
    );
    let cfg: Config =
        toml::from_str(&toml_str).map_err(|e| AshError::InvalidConfig(e.to_string()))?;
    // Validate by calling adapter_kind — if it fails, the config is invalid.
    cfg.adapter_kind()?;
    Ok(cfg)
}

#[test]
fn adapter_kind_openai() {
    assert!(make_config("openai").is_ok());
}

#[test]
fn adapter_kind_anthropic() {
    assert!(make_config("anthropic").is_ok());
}

#[test]
fn adapter_kind_gemini() {
    assert!(make_config("gemini").is_ok());
}

#[test]
fn adapter_kind_ollama() {
    assert!(make_config("ollama").is_ok());
}

#[test]
fn adapter_kind_deepseek() {
    assert!(make_config("deepseek").is_ok());
}

#[test]
fn adapter_kind_xai() {
    assert!(make_config("xai").is_ok());
}

#[test]
fn adapter_kind_case_insensitive() {
    assert!(make_config("OpenAI").is_ok());
    assert!(make_config("ANTHROPIC").is_ok());
}

#[test]
fn adapter_kind_invalid() {
    assert!(make_config("foobar").is_err());
}

#[test]
fn config_omits_provider_name() {
    // provider_name field was removed — config without it must parse.
    let toml_str = r#"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#;
    let cfg: Config = toml::from_str(toml_str).expect("must parse without provider_name");
    assert_eq!(cfg.api_type, "openai");
}

#[test]
fn config_api_key_defaults_empty() {
    let toml_str = r#"
api_type = "openai"
model_name = "m"
allow_list = []
deny_list = []
"#;
    let cfg: Config = toml::from_str(toml_str).expect("must parse with omitted api_key");
    assert_eq!(cfg.api_key, "");
}

#[test]
fn config_request_timeout_defaults_60() {
    let toml_str = r#"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.request_timeout_secs, 60);
}

#[test]
fn config_tools_to_probe_defaults() {
    let toml_str = r#"
api_type = "openai"
api_key = "k"
model_name = "m"
allow_list = []
deny_list = []
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(cfg.tools_to_probe.len(), 7);
    assert!(cfg.tools_to_probe.contains(&"git".to_string()));
}
