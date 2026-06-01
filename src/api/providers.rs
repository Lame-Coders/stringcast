use super::response_cleaner::clean_response;
use serde_json::{json, Value};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Gemini,
    OpenAi,
    Anthropic,
    CustomOpenAiCompatible,
}

impl ProviderKind {
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::CustomOpenAiCompatible => "custom",
        }
    }
}

impl FromStr for ProviderKind {
    type Err = ProviderError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "custom" => Ok(Self::CustomOpenAiCompatible),
            _ => Err(ProviderError::UnknownProvider(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequestConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub api_key: String,
    pub custom_base_url: Option<String>,
    pub system_prompt: String,
    pub user_text: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    MissingCustomBaseUrl,
    MalformedResponse,
    EmptyResponse,
    UnknownProvider(String),
}

impl ProviderRequestConfig {
    pub fn build_request(&self) -> Result<ProviderRequest, ProviderError> {
        match self.provider {
            ProviderKind::Gemini => Ok(self.build_gemini_request()),
            ProviderKind::OpenAi => Ok(self.build_openai_request("https://api.openai.com/v1")),
            ProviderKind::Anthropic => Ok(self.build_anthropic_request()),
            ProviderKind::CustomOpenAiCompatible => {
                let base_url = self
                    .custom_base_url
                    .as_deref()
                    .filter(|url| !url.trim().is_empty())
                    .ok_or(ProviderError::MissingCustomBaseUrl)?;
                Ok(self.build_openai_request(base_url.trim_end_matches('/')))
            }
        }
    }

    fn build_gemini_request(&self) -> ProviderRequest {
        ProviderRequest {
            url: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                self.model, self.api_key
            ),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: json!({
                "contents": [{
                    "role": "user",
                    "parts": [{ "text": self.user_text }]
                }],
                "systemInstruction": {
                    "parts": [{ "text": self.system_prompt }]
                },
                "generationConfig": {
                    "temperature": self.temperature,
                    "maxOutputTokens": self.max_output_tokens,
                    "candidateCount": 1
                },
                "safetySettings": [
                    {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
                    {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
                    {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
                    {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"}
                ]
            }),
        }
    }

    fn build_openai_request(&self, base_url: &str) -> ProviderRequest {
        ProviderRequest {
            url: format!("{base_url}/chat/completions"),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                (
                    "Authorization".to_string(),
                    format!("Bearer {}", self.api_key),
                ),
            ],
            body: json!({
                "model": self.model,
                "messages": [
                    { "role": "system", "content": self.system_prompt },
                    { "role": "user", "content": self.user_text }
                ],
                "temperature": self.temperature,
                "max_tokens": self.max_output_tokens,
                "n": 1
            }),
        }
    }

    fn build_anthropic_request(&self) -> ProviderRequest {
        ProviderRequest {
            url: "https://api.anthropic.com/v1/messages".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-api-key".to_string(), self.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ],
            body: json!({
                "model": self.model,
                "max_tokens": self.max_output_tokens,
                "system": self.system_prompt,
                "messages": [
                    { "role": "user", "content": self.user_text }
                ]
            }),
        }
    }
}

pub fn parse_provider_response(
    provider: ProviderKind,
    body: &str,
    raw_output: bool,
) -> Result<String, ProviderError> {
    let parsed: Value = serde_json::from_str(body).map_err(|_| ProviderError::MalformedResponse)?;
    let text = match provider {
        ProviderKind::Gemini => parsed
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str),
        ProviderKind::OpenAi | ProviderKind::CustomOpenAiCompatible => parsed
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str),
        ProviderKind::Anthropic => parsed.pointer("/content/0/text").and_then(Value::as_str),
    }
    .ok_or(ProviderError::MalformedResponse)?;

    let cleaned = clean_response(text, raw_output);
    if cleaned.trim().is_empty() {
        return Err(ProviderError::EmptyResponse);
    }

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config(provider: ProviderKind) -> ProviderRequestConfig {
        ProviderRequestConfig {
            provider,
            model: "model".to_string(),
            api_key: "secret".to_string(),
            custom_base_url: None,
            system_prompt: "Fix text.".to_string(),
            user_text: "helo".to_string(),
            temperature: 0.3,
            max_output_tokens: 2048,
        }
    }

    #[test]
    fn builds_openai_chat_completion_request() {
        let request = base_config(ProviderKind::OpenAi).build_request().unwrap();

        assert_eq!(request.url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(request.body["messages"][0]["role"], "system");
        assert_eq!(request.body["messages"][1]["content"], "helo");
    }

    #[test]
    fn custom_provider_requires_base_url() {
        let error = base_config(ProviderKind::CustomOpenAiCompatible)
            .build_request()
            .unwrap_err();

        assert_eq!(error, ProviderError::MissingCustomBaseUrl);
    }

    #[test]
    fn parses_gemini_text_response() {
        let body = r#"{
            "candidates": [{
                "content": { "parts": [{ "text": "Hello." }] }
            }]
        }"#;

        let parsed = parse_provider_response(ProviderKind::Gemini, body, false).unwrap();

        assert_eq!(parsed, "Hello.");
    }

    #[test]
    fn parses_openai_text_response_and_cleans_it() {
        let body = r#"{
            "choices": [{
                "message": { "content": "Sure,\nHello." }
            }]
        }"#;

        let parsed = parse_provider_response(ProviderKind::OpenAi, body, false).unwrap();

        assert_eq!(parsed, "Hello.");
    }

    #[test]
    fn parses_anthropic_text_response() {
        let body = r#"{
            "content": [{ "type": "text", "text": "Hello." }]
        }"#;

        let parsed = parse_provider_response(ProviderKind::Anthropic, body, false).unwrap();

        assert_eq!(parsed, "Hello.");
    }
}
