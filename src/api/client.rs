use super::{
    parse_provider_response, KeyPool, ProviderError, ProviderKind, ProviderRequest,
    ProviderRequestConfig,
};
use crate::storage::AppConfig;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct ApiClientConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub custom_base_url: Option<String>,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub max_attempts: u8,
}

impl TryFrom<&AppConfig> for ApiClientConfig {
    type Error = ProviderError;

    fn try_from(config: &AppConfig) -> Result<Self, Self::Error> {
        let provider = ProviderKind::from_str(&config.provider.active)?;
        let model = match provider {
            ProviderKind::Gemini => config.provider.gemini_model.clone(),
            ProviderKind::OpenAi => config.provider.openai_model.clone(),
            ProviderKind::Anthropic => config.provider.anthropic_model.clone(),
            ProviderKind::CustomOpenAiCompatible => config.provider.custom_model.clone(),
        };

        Ok(Self {
            provider,
            model,
            custom_base_url: (!config.provider.custom_base_url.trim().is_empty())
                .then(|| config.provider.custom_base_url.clone()),
            temperature: config.api.temperature,
            max_output_tokens: config.api.max_output_tokens,
            max_attempts: config.api.max_retries.max(1),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Network,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiClientError {
    NoKeysAvailable,
    MissingKeyMaterial(String),
    Provider(ProviderError),
    Transport(TransportError),
    RetryExhausted,
    BadRequest(Option<String>),
    UnexpectedStatus {
        status: u16,
        message: Option<String>,
    },
}

pub trait HttpTransport {
    fn send(&mut self, request: ProviderRequest) -> Result<HttpResponse, TransportError>;
}

pub trait KeyMaterialStore {
    fn key_material(&self, key_id: &str) -> Option<String>;
}

#[derive(Debug, Clone, Default)]
pub struct StaticKeyMaterialStore {
    keys: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct EnvKeyMaterialStore;

impl StaticKeyMaterialStore {
    pub fn new(keys: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }
}

impl KeyMaterialStore for StaticKeyMaterialStore {
    fn key_material(&self, key_id: &str) -> Option<String> {
        self.keys.get(key_id).cloned()
    }
}

impl KeyMaterialStore for EnvKeyMaterialStore {
    fn key_material(&self, key_id: &str) -> Option<String> {
        let normalized = key_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();

        env::var(format!("STRINGCAST_KEY_{normalized}"))
            .ok()
            .or_else(|| env::var("STRINGCAST_API_KEY").ok())
    }
}

#[derive(Debug)]
pub struct ApiClient<T, K> {
    config: ApiClientConfig,
    key_pool: KeyPool,
    transport: T,
    key_material: K,
}

impl<T, K> ApiClient<T, K>
where
    T: HttpTransport,
    K: KeyMaterialStore,
{
    pub fn new(config: ApiClientConfig, key_pool: KeyPool, transport: T, key_material: K) -> Self {
        Self {
            config,
            key_pool,
            transport,
            key_material,
        }
    }

    pub fn key_pool(&self) -> &KeyPool {
        &self.key_pool
    }

    pub fn transform(
        &mut self,
        system_prompt: &str,
        user_text: &str,
        raw_output: bool,
        now: Instant,
    ) -> Result<String, ApiClientError> {
        let max_attempts = self.config.max_attempts.max(1);
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_attempts {
            attempts += 1;
            let Some(key) = self
                .key_pool
                .next_available_key(
                    self.config.provider,
                    now + Duration::from_millis(attempts as u64),
                )
                .cloned()
            else {
                return Err(last_error.unwrap_or(ApiClientError::NoKeysAvailable));
            };

            let Some(api_key) = self.key_material.key_material(&key.id) else {
                self.key_pool.mark_invalid(&key.id);
                last_error = Some(ApiClientError::MissingKeyMaterial(key.id));
                continue;
            };

            let request = ProviderRequestConfig {
                provider: self.config.provider,
                model: self.config.model.clone(),
                api_key,
                custom_base_url: self.config.custom_base_url.clone(),
                system_prompt: system_prompt.to_string(),
                user_text: user_text.to_string(),
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_output_tokens,
            }
            .build_request()
            .map_err(ApiClientError::Provider)?;

            let response = match self.transport.send(request) {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(ApiClientError::Transport(error));
                    continue;
                }
            };

            match response.status {
                200 => {
                    let parsed =
                        parse_provider_response(self.config.provider, &response.body, raw_output)
                            .map_err(ApiClientError::Provider)?;
                    self.key_pool.mark_success(&key.id);
                    return Ok(parsed);
                }
                400 => {
                    return Err(ApiClientError::BadRequest(provider_error_message(
                        &response,
                    )))
                }
                401 | 403 => {
                    self.key_pool.mark_invalid(&key.id);
                    last_error = Some(ApiClientError::UnexpectedStatus {
                        status: response.status,
                        message: provider_error_message(&response),
                    });
                }
                429 => {
                    let retry_after =
                        retry_after_duration(&response).unwrap_or_else(|| Duration::from_secs(60));
                    self.key_pool.mark_rate_limited(&key.id, now + retry_after);
                    last_error = Some(ApiClientError::UnexpectedStatus {
                        status: 429,
                        message: provider_error_message(&response),
                    });
                }
                500 | 502 | 503 | 504 => {
                    last_error = Some(ApiClientError::UnexpectedStatus {
                        status: response.status,
                        message: provider_error_message(&response),
                    });
                }
                status => {
                    return Err(ApiClientError::UnexpectedStatus {
                        status,
                        message: provider_error_message(&response),
                    })
                }
            }
        }

        Err(last_error.unwrap_or(ApiClientError::RetryExhausted))
    }
}

fn provider_error_message(response: &HttpResponse) -> Option<String> {
    let body = response.body.trim();
    if body.is_empty() {
        return None;
    }

    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.pointer("/message"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.to_string());

    Some(message.chars().take(500).collect())
}

impl std::fmt::Display for ApiClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKeysAvailable => write!(formatter, "no active API keys are available"),
            Self::MissingKeyMaterial(key_id) => {
                write!(formatter, "key material is missing for key id '{key_id}'")
            }
            Self::Provider(error) => write!(formatter, "provider response error: {error:?}"),
            Self::Transport(error) => write!(formatter, "transport error: {error:?}"),
            Self::RetryExhausted => write!(formatter, "retry attempts exhausted"),
            Self::BadRequest(message) => write_status_error(formatter, 400, message),
            Self::UnexpectedStatus { status, message } => {
                write_status_error(formatter, *status, message)
            }
        }
    }
}

fn write_status_error(
    formatter: &mut std::fmt::Formatter<'_>,
    status: u16,
    message: &Option<String>,
) -> std::fmt::Result {
    match message {
        Some(message) => write!(formatter, "provider returned HTTP {status}: {message}"),
        None => write!(formatter, "provider returned HTTP {status}"),
    }
}

impl std::error::Error for ApiClientError {}

fn retry_after_duration(response: &HttpResponse) -> Option<Duration> {
    response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, value)| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApiKey, KeyStatus};

    #[derive(Debug)]
    struct FakeTransport {
        responses: Vec<Result<HttpResponse, TransportError>>,
        sent_urls: Vec<String>,
    }

    impl HttpTransport for FakeTransport {
        fn send(&mut self, request: ProviderRequest) -> Result<HttpResponse, TransportError> {
            self.sent_urls.push(request.url);
            self.responses.remove(0)
        }
    }

    fn api_key(id: &str) -> ApiKey {
        ApiKey {
            id: id.to_string(),
            provider: ProviderKind::OpenAi,
            alias: None,
            status: KeyStatus::Active,
            rate_limit_until: None,
            consecutive_errors: 0,
            last_used: None,
            requests_total: 0,
            requests_success: 0,
        }
    }

    fn config() -> ApiClientConfig {
        ApiClientConfig {
            provider: ProviderKind::OpenAi,
            model: "gpt-test".to_string(),
            custom_base_url: None,
            temperature: 0.3,
            max_output_tokens: 2048,
            max_attempts: 3,
        }
    }

    #[test]
    fn builds_client_config_from_app_config() {
        let mut app_config = AppConfig::default();
        app_config.provider.active = "anthropic".to_string();
        app_config.provider.anthropic_model = "claude-test".to_string();
        app_config.api.max_retries = 2;

        let client_config = ApiClientConfig::try_from(&app_config).unwrap();

        assert_eq!(client_config.provider, ProviderKind::Anthropic);
        assert_eq!(client_config.model, "claude-test");
        assert_eq!(client_config.max_attempts, 2);
    }

    #[test]
    fn transforms_successful_response() {
        let transport = FakeTransport {
            responses: vec![Ok(HttpResponse {
                status: 200,
                headers: vec![],
                body: r#"{"choices":[{"message":{"content":"Hello."}}]}"#.to_string(),
            })],
            sent_urls: vec![],
        };
        let key_material =
            StaticKeyMaterialStore::new([("key-1".to_string(), "secret".to_string())]);
        let mut client = ApiClient::new(
            config(),
            KeyPool::new(vec![api_key("key-1")]),
            transport,
            key_material,
        );

        let output = client
            .transform("Fix text.", "helo", false, Instant::now())
            .unwrap();

        assert_eq!(output, "Hello.");
        assert_eq!(client.key_pool().keys()[0].requests_success, 1);
    }

    #[test]
    fn rotates_after_rate_limit() {
        let transport = FakeTransport {
            responses: vec![
                Ok(HttpResponse {
                    status: 429,
                    headers: vec![("Retry-After".to_string(), "120".to_string())],
                    body: String::new(),
                }),
                Ok(HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: r#"{"choices":[{"message":{"content":"Hello."}}]}"#.to_string(),
                }),
            ],
            sent_urls: vec![],
        };
        let key_material = StaticKeyMaterialStore::new([
            ("key-1".to_string(), "secret-1".to_string()),
            ("key-2".to_string(), "secret-2".to_string()),
        ]);
        let mut client = ApiClient::new(
            config(),
            KeyPool::new(vec![api_key("key-1"), api_key("key-2")]),
            transport,
            key_material,
        );

        let output = client
            .transform("Fix text.", "helo", false, Instant::now())
            .unwrap();

        assert_eq!(output, "Hello.");
        assert_eq!(client.key_pool().keys()[0].status, KeyStatus::RateLimited);
        assert_eq!(client.key_pool().keys()[1].requests_success, 1);
    }

    #[test]
    fn marks_unauthorized_key_invalid() {
        let transport = FakeTransport {
            responses: vec![Ok(HttpResponse {
                status: 401,
                headers: vec![],
                body: String::new(),
            })],
            sent_urls: vec![],
        };
        let key_material =
            StaticKeyMaterialStore::new([("key-1".to_string(), "secret".to_string())]);
        let mut client = ApiClient::new(
            ApiClientConfig {
                max_attempts: 1,
                ..config()
            },
            KeyPool::new(vec![api_key("key-1")]),
            transport,
            key_material,
        );

        let result = client.transform("Fix text.", "helo", false, Instant::now());

        assert_eq!(
            result,
            Err(ApiClientError::UnexpectedStatus {
                status: 401,
                message: None
            })
        );
        assert_eq!(client.key_pool().keys()[0].status, KeyStatus::Invalid);
    }

    #[test]
    fn includes_provider_error_message() {
        let transport = FakeTransport {
            responses: vec![Ok(HttpResponse {
                status: 400,
                headers: vec![],
                body: r#"{"error":{"message":"API key not valid"}}"#.to_string(),
            })],
            sent_urls: vec![],
        };
        let key_material =
            StaticKeyMaterialStore::new([("key-1".to_string(), "secret".to_string())]);
        let mut client = ApiClient::new(
            ApiClientConfig {
                max_attempts: 1,
                ..config()
            },
            KeyPool::new(vec![api_key("key-1")]),
            transport,
            key_material,
        );

        let result = client.transform("Fix text.", "helo", false, Instant::now());

        assert_eq!(
            result,
            Err(ApiClientError::BadRequest(Some(
                "API key not valid".to_string()
            )))
        );
    }
}
