use crate::config::Config;
use crate::env_probe::EnvSummary;
use crate::error::AshError;
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest, Usage};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LlmDraft {
    pub command: String,
    pub explanation: Option<String>,
}

pub async fn generate(
    prompt: &str,
    env: &EnvSummary,
    cfg: &Config,
) -> Result<(LlmDraft, Usage), AshError> {
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

    let response = client
        .exec_chat(&cfg.model_name, req, None)
        .await
        .map_err(|e| AshError::NetworkError(e.to_string()))?;

    let usage = response.usage.clone();
    let text = response.first_text().unwrap_or("").trim().to_string();
    let draft = parse_draft(&text)?;
    Ok((draft, usage))
}

fn parse_draft(text: &str) -> Result<LlmDraft, AshError> {
    // Locate the outermost { … } span. rfind('}') is intentional: it finds the last
    // closing brace so that any trailing prose ("Here you go!") after the JSON is excluded,
    // while still correctly handling shell brace expansion inside string values.
    let no_json = || AshError::LlmOutputError("no JSON object in response".into());
    let start = text.find('{').ok_or_else(no_json)?;
    let end = text.rfind('}').ok_or_else(no_json)?;
    let json = &text[start..=end];

    let draft: LlmDraft = serde_json::from_str(json)
        .map_err(|e| AshError::LlmOutputError(format!("invalid JSON: {e}")))?;

    if draft.command.trim().is_empty() {
        return Err(AshError::LlmOutputError("command is empty".into()));
    }
    Ok(draft)
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
    use super::parse_draft;
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
}
