use crate::config::Config;
use crate::env_probe::EnvSummary;
use crate::error::AshError;
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use serde::Deserialize;
use tokio::time::{timeout, Duration};

#[derive(Debug, Deserialize)]
pub struct LlmDraft {
    pub command: String,
    pub explanation: Option<String>,
}

/// Provider-neutral token accounting used by the UI. Keeping this type local
/// avoids coupling callers to a specific provider SDK's usage schema.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

pub async fn generate(
    prompt: &str,
    env: &EnvSummary,
    cfg: &Config,
) -> Result<(LlmDraft, TokenUsage), AshError> {
    if use_responses_api(cfg) {
        return generate_responses(prompt, env, cfg).await;
    }

    let client = build_client(cfg)?;
    let system = format!(
        "You are an expert CLI assistant. Your sole purpose is to translate the user's natural language request into a valid shell command based on their environment.\n\n\
        [ENVIRONMENT CONTEXT]\n\
        {}\n\n\
        [OUTPUT FORMAT]\n\
        You MUST respond with a single, raw, valid JSON object. Do NOT wrap the response in Markdown code blocks (e.g., ```json), and do NOT include any introductory or concluding text. \n\
        The JSON structure must strictly follow this schema:\n\
        {{\n\
        \"command\": \"The exact shell command to execute.\",\n\
        \"explanation\": \"A concise, single-sentence explanation of what the command does.\"\n\
        }}\n\n\
        [RULES & CONSTRAINTS]\n\
        1. Cross-Platform Accuracy: Tailor the command to the OS, shell, and package managers detected in the environment context (e.g., adapt to apt/yum/apk/brew).\n\
        2. Safety First: Do not generate inherently destructive or malicious commands unless explicitly and unambiguously requested.\n\
        3. Formatting: Ensure all quotes inside the \"command\" string are properly escaped for JSON safety.\n\
        4. Adaptive Localization: The \"explanation\" MUST be written in the exact same language as the user's input request. For example, if the user asks in Chinese, explain in Chinese; if they ask in English, explain in English.",
        env.text
    );
    let req = ChatRequest::new(vec![ChatMessage::system(system), ChatMessage::user(prompt)]);

    let response = timeout(
        Duration::from_secs(cfg.request_timeout_secs),
        client.exec_chat(&cfg.model_name, req, None),
    )
    .await
    .map_err(|_| {
        AshError::Timeout(format!(
            "request timed out after {}s",
            cfg.request_timeout_secs
        ))
    })?
    .map_err(|e| AshError::NetworkError(e.to_string()))?;

    let usage = response.usage.clone();
    let text = response
        .first_text()
        .ok_or_else(|| AshError::LlmOutputError("no text in response".into()))?
        .trim()
        .to_string();
    let draft = parse_draft(&text)?;
    Ok((
        draft,
        TokenUsage {
            prompt_tokens: usage.prompt_tokens.map(|v| v as u64),
            completion_tokens: usage.completion_tokens.map(|v| v as u64),
            total_tokens: usage.total_tokens.map(|v| v as u64),
        },
    ))
}

fn use_responses_api(cfg: &Config) -> bool {
    openai_api_mode(
        &cfg.api_type,
        &cfg.model_name,
        std::env::var("ASH_OPENAI_API_MODE").ok().as_deref(),
    )
}

fn openai_api_mode(api_type: &str, model_name: &str, override_mode: Option<&str>) -> bool {
    if !api_type.eq_ignore_ascii_case("openai") {
        return false;
    }
    match override_mode {
        Some("chat") => false,
        Some("responses") => true,
        _ => model_name.to_ascii_lowercase().starts_with("gpt-5"),
    }
}

async fn generate_responses(
    prompt: &str,
    env: &EnvSummary,
    cfg: &Config,
) -> Result<(LlmDraft, TokenUsage), AshError> {
    let system = format!(
        "You are an expert CLI assistant. Translate the user's request into one valid shell command for the detected environment. Return only the JSON object required by the response schema. Never add Markdown or prose.\n\nENVIRONMENT:\n{}",
        env.text
    );
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "command": { "type": "string" },
            "explanation": { "type": ["string", "null"] }
        },
        "required": ["command", "explanation"]
    });
    let mut body = serde_json::json!({
        "model": cfg.model_name.as_str(),
        "instructions": system,
        "input": prompt,
        "text": { "format": { "type": "json_schema", "name": "shell_command", "strict": true, "schema": schema } }
    });
    if let Some(effort) = std::env::var("ASH_REASONING_EFFORT").ok().filter(|v| !v.is_empty()) {
        body["reasoning"] = serde_json::json!({ "effort": effort });
    }
    let endpoint = cfg
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1")
        .trim_end_matches('/')
        .to_owned()
        + "/responses";
    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(cfg.request_timeout_secs),
        client.post(endpoint).bearer_auth(&cfg.api_key).json(&body).send(),
    )
    .await
    .map_err(|_| AshError::Timeout(format!("request timed out after {}s", cfg.request_timeout_secs)))?
    .map_err(|e| AshError::NetworkError(e.to_string()))?;
    let status = response.status();
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AshError::NetworkError(format!("invalid API response: {e}")))?;
    if !status.is_success() {
        let message = payload["error"]["message"]
            .as_str()
            .unwrap_or("OpenAI Responses API request failed");
        return Err(AshError::NetworkError(format!("HTTP {status}: {message}")));
    }
    parse_responses_payload(&payload)
}

fn parse_responses_payload(payload: &serde_json::Value) -> Result<(LlmDraft, TokenUsage), AshError> {
    let text = payload["output_text"]
        .as_str()
        .or_else(|| {
            payload["output"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|item| item["content"].as_array().into_iter().flatten())
                .find_map(|part| part["text"].as_str())
        })
        .ok_or_else(|| AshError::LlmOutputError("no text in Responses API output".into()))?;
    let draft = parse_draft(text.trim())?;
    let usage = TokenUsage {
        prompt_tokens: payload["usage"]["input_tokens"].as_u64(),
        completion_tokens: payload["usage"]["output_tokens"].as_u64(),
        total_tokens: payload["usage"]["total_tokens"].as_u64(),
    };
    Ok((draft, usage))
}

fn parse_draft(text: &str) -> Result<LlmDraft, AshError> {
    let no_json = || AshError::LlmOutputError("no JSON object in response".into());
    let start = text.find('{').ok_or_else(no_json)?;
    let json = extract_json(&text[start..]).ok_or_else(no_json)?;
    let draft: LlmDraft = serde_json::from_str(json)
        .map_err(|e| AshError::LlmOutputError(format!("invalid JSON: {e}")))?;
    if draft.command.trim().is_empty() {
        return Err(AshError::LlmOutputError("command is empty".into()));
    }
    Ok(draft)
}

/// Extract the first complete JSON object span using a brace-depth counter
/// that respects JSON string escaping. More robust than rfind('}') which
/// breaks on trailing prose containing literal braces.
fn extract_json(s: &str) -> Option<&str> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0;
    let mut found_start = false;
    for (i, c) in s.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = i;
                    found_start = true;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 && found_start {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn build_client(cfg: &Config) -> Result<Client, AshError> {
    let api_key = cfg.api_key.clone();
    let base_url = cfg.base_url.clone();
    let adapter = cfg.adapter_kind()?;

    let client = Client::builder()
        .with_adapter_kind(adapter)
        .with_service_target_resolver(ServiceTargetResolver::from_resolver_fn(
            move |mut st: genai::ServiceTarget| {
                st.auth = AuthData::from_single(api_key.clone());
                if let Some(url) = &base_url {
                    st.endpoint = Endpoint::from_owned(url.clone());
                }
                Ok(st)
            },
        ))
        .build();

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::{openai_api_mode, parse_draft, parse_responses_payload};
    use crate::error::AshError;

    #[test]
    fn valid_json_parses() {
        let d = parse_draft(r#"{"command":"git status","explanation":"show status"}"#).unwrap();
        assert_eq!(d.command, "git status");
    }

    #[test]
    fn extra_fields_ignored() {
        let d = parse_draft(r#"{"command":"ls","foo":"bar"}"#).unwrap();
        assert_eq!(d.command, "ls");
    }

    #[test]
    fn empty_command_fails() {
        assert!(matches!(
            parse_draft(r#"{"command":""}"#),
            Err(AshError::LlmOutputError(_))
        ));
    }

    #[test]
    fn invalid_json_fails() {
        assert!(matches!(
            parse_draft("not json"),
            Err(AshError::LlmOutputError(_))
        ));
    }

    #[test]
    fn code_fences_stripped() {
        let d = parse_draft("```json\n{\"command\":\"ls\"}\n```").unwrap();
        assert_eq!(d.command, "ls");
    }

    #[test]
    fn trailing_prose_stripped() {
        let d = parse_draft(r#"Here you go: {"command":"ls"} Hope that helps!"#).unwrap();
        assert_eq!(d.command, "ls");
    }

    #[test]
    fn trailing_prose_with_brace_stripped() {
        // Trailing prose containing a literal } must not confuse the parser.
        let d = parse_draft(r#"{"command":"ls"} oops } done"#).unwrap();
        assert_eq!(d.command, "ls");
    }

    #[test]
    fn nested_braces_in_string() {
        let d = parse_draft(r#"{"command":"echo {test}","explanation":"x"}"#).unwrap();
        assert_eq!(d.command, "echo {test}");
    }

    #[test]
    fn escaped_quote_in_string() {
        let d = parse_draft(r#"{"command":"echo \"hello\"","explanation":"x"}"#).unwrap();
        assert_eq!(d.command, r#"echo "hello""#);
    }

    #[test]
    fn gpt5_openai_uses_responses_by_default() {
        assert!(openai_api_mode("openai", "gpt-5.6", None));
        assert!(!openai_api_mode("deepseek", "gpt-5.6", None));
        assert!(!openai_api_mode("openai", "gpt-4o-mini", None));
        assert!(!openai_api_mode("openai", "gpt-5.6", Some("chat")));
        assert!(openai_api_mode("openai", "gpt-4o-mini", Some("responses")));
    }

    #[test]
    fn responses_payload_parses_output_and_usage() {
        let payload = serde_json::json!({
            "output": [{
                "content": [{"type": "output_text", "text": r#"{"command":"git status","explanation":"show status"}"#}]
            }],
            "usage": {"input_tokens": 12, "output_tokens": 8, "total_tokens": 20}
        });
        let (draft, usage) = parse_responses_payload(&payload).unwrap();
        assert_eq!(draft.command, "git status");
        assert_eq!(usage.prompt_tokens, Some(12));
        assert_eq!(usage.completion_tokens, Some(8));
        assert_eq!(usage.total_tokens, Some(20));
    }
}
