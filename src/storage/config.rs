use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    ParseToml(toml::de::Error),
    SerializeToml(toml::ser::Error),
    Validation(String),
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::ParseToml(error)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(error: toml::ser::Error) -> Self {
        Self::SerializeToml(error)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub extraction: ExtractionConfig,
    #[serde(default)]
    pub exclusions: ExclusionsConfig,
    #[serde(default)]
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub commands: CommandsConfig,
    #[serde(default)]
    pub api_keys: Vec<ApiKeyConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub enabled: bool,
    pub startup_at_login: bool,
    pub show_spinner: bool,
    pub spinner_style: String,
    pub max_extract_chars: usize,
    pub undo_timeout_minutes: u32,
    pub log_level: String,
    pub collect_local_stats: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub active: String,
    pub gemini_model: String,
    pub openai_model: String,
    pub anthropic_model: String,
    pub custom_base_url: String,
    pub custom_model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiConfig {
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub connection_timeout_ms: u64,
    pub response_timeout_ms: u64,
    pub max_retries: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub select_all_wait_ms: u64,
    pub clipboard_read_wait_ms: u64,
    pub clipboard_restore_wait_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExclusionsConfig {
    #[serde(default)]
    pub apps: Vec<String>,
    #[serde(default)]
    pub known_problematic_apps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub confirm_before_first_api_call: bool,
    pub redact_logs: bool,
    pub allow_debug_body_logging: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommandsConfig {
    #[serde(default)]
    pub disabled_builtins: Vec<String>,
    #[serde(default)]
    pub custom: Vec<CommandConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandConfig {
    pub trigger: String,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub case_sensitive: bool,
    pub raw_output: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub id: String,
    pub provider: String,
    pub alias: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: default_config_version(),
            general: GeneralConfig::default(),
            provider: ProviderConfig::default(),
            api: ApiConfig::default(),
            extraction: ExtractionConfig::default(),
            exclusions: ExclusionsConfig::default(),
            privacy: PrivacyConfig::default(),
            commands: CommandsConfig::default(),
            api_keys: Vec::new(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            startup_at_login: true,
            show_spinner: true,
            spinner_style: "dots".to_string(),
            max_extract_chars: 16_384,
            undo_timeout_minutes: 10,
            log_level: "warn".to_string(),
            collect_local_stats: true,
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            active: "gemini".to_string(),
            gemini_model: "gemini-2.0-flash-lite".to_string(),
            openai_model: "gpt-4o-mini".to_string(),
            anthropic_model: "claude-haiku-4-5".to_string(),
            custom_base_url: String::new(),
            custom_model: String::new(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            temperature: 0.3,
            max_output_tokens: 2048,
            connection_timeout_ms: 5000,
            response_timeout_ms: 30_000,
            max_retries: 3,
        }
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            select_all_wait_ms: 100,
            clipboard_read_wait_ms: 300,
            clipboard_restore_wait_ms: 150,
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            confirm_before_first_api_call: true,
            redact_logs: true,
            allow_debug_body_logging: false,
        }
    }
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save_atomic(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        self.validate()?;

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let tmp_path = path.with_extension("toml.tmp");
        let serialized = toml::to_string_pretty(self)?;
        fs::write(&tmp_path, serialized)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_one_of(
            "general.spinner_style",
            &self.general.spinner_style,
            &["dots", "braille", "bar"],
        )?;
        validate_range(
            "general.max_extract_chars",
            self.general.max_extract_chars,
            1024,
            16_384,
        )?;
        validate_range(
            "general.undo_timeout_minutes",
            self.general.undo_timeout_minutes,
            1,
            60,
        )?;
        validate_one_of(
            "general.log_level",
            &self.general.log_level,
            &["error", "warn", "info", "debug"],
        )?;
        validate_one_of(
            "provider.active",
            &self.provider.active,
            &["gemini", "openai", "anthropic", "custom"],
        )?;

        if !(0.0..=1.0).contains(&self.api.temperature) {
            return Err(ConfigError::Validation(
                "api.temperature must be between 0.0 and 1.0".to_string(),
            ));
        }
        validate_range(
            "api.max_output_tokens",
            self.api.max_output_tokens,
            256,
            4096,
        )?;
        validate_range(
            "api.connection_timeout_ms",
            self.api.connection_timeout_ms,
            1000,
            30_000,
        )?;
        validate_range(
            "api.response_timeout_ms",
            self.api.response_timeout_ms,
            10_000,
            120_000,
        )?;
        validate_range("api.max_retries", self.api.max_retries, 0, 10)?;

        if self.privacy.allow_debug_body_logging && !self.privacy.redact_logs {
            return Err(ConfigError::Validation(
                "privacy.allow_debug_body_logging requires privacy.redact_logs = true".to_string(),
            ));
        }

        Ok(())
    }

    pub fn upsert_api_key(&mut self, key: ApiKeyConfig) {
        if let Some(existing) = self
            .api_keys
            .iter_mut()
            .find(|existing| existing.id == key.id)
        {
            *existing = key;
        } else {
            self.api_keys.push(key);
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.general.enabled = enabled;
    }

    pub fn set_provider(&mut self, provider: &str) -> Result<(), ConfigError> {
        validate_one_of(
            "provider.active",
            provider,
            &["gemini", "openai", "anthropic", "custom"],
        )?;
        self.provider.active = provider.to_string();
        Ok(())
    }

    pub fn active_provider_key_count(&self) -> usize {
        self.api_keys
            .iter()
            .filter(|key| {
                key.provider == self.provider.active && key.status.eq_ignore_ascii_case("active")
            })
            .count()
    }
}

fn default_config_version() -> u32 {
    1
}

fn validate_one_of(name: &str, value: &str, allowed: &[&str]) -> Result<(), ConfigError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ConfigError::Validation(format!(
            "{name} must be one of: {}",
            allowed.join(", ")
        )))
    }
}

fn validate_range<T>(name: &str, value: T, min: T, max: T) -> Result<(), ConfigError>
where
    T: PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        return Err(ConfigError::Validation(format!(
            "{name} must be between {min} and {max}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_config_matches_spec_values() {
        let config = AppConfig::default();

        assert_eq!(config.config_version, 1);
        assert_eq!(config.general.max_extract_chars, 16_384);
        assert_eq!(config.provider.active, "gemini");
        assert_eq!(config.api.max_retries, 3);
        assert!(config.privacy.confirm_before_first_api_call);
    }

    #[test]
    fn invalid_config_is_rejected() {
        let mut config = AppConfig::default();
        config.general.spinner_style = "orbit".to_string();

        assert!(matches!(config.validate(), Err(ConfigError::Validation(_))));
    }

    #[test]
    fn config_round_trips_through_toml_file() {
        let path = std::env::temp_dir().join(format!(
            "stringcast-config-test-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut config = AppConfig::default();
        config.exclusions.apps.push("1Password.exe".to_string());

        config.save_atomic(&path).unwrap();
        let loaded = AppConfig::load(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn upsert_api_key_replaces_existing_metadata() {
        let mut config = AppConfig::default();
        config.upsert_api_key(ApiKeyConfig {
            id: "key-1".to_string(),
            provider: "openai".to_string(),
            alias: Some("Old".to_string()),
            status: "Active".to_string(),
            created_at: "2026-05-31T00:00:00Z".to_string(),
        });
        config.upsert_api_key(ApiKeyConfig {
            id: "key-1".to_string(),
            provider: "openai".to_string(),
            alias: Some("New".to_string()),
            status: "Active".to_string(),
            created_at: "2026-06-01T00:00:00Z".to_string(),
        });

        assert_eq!(config.api_keys.len(), 1);
        assert_eq!(config.api_keys[0].alias.as_deref(), Some("New"));
    }

    #[test]
    fn can_toggle_enabled_and_set_provider() {
        let mut config = AppConfig::default();

        config.set_enabled(false);
        config.set_provider("openai").unwrap();

        assert!(!config.general.enabled);
        assert_eq!(config.provider.active, "openai");
    }

    #[test]
    fn active_provider_key_count_counts_only_active_matching_keys() {
        let mut config = AppConfig::default();
        config.provider.active = "openai".to_string();
        config.api_keys = vec![
            ApiKeyConfig {
                id: "openai-active".to_string(),
                provider: "openai".to_string(),
                alias: None,
                status: "Active".to_string(),
                created_at: "0".to_string(),
            },
            ApiKeyConfig {
                id: "openai-disabled".to_string(),
                provider: "openai".to_string(),
                alias: None,
                status: "Disabled".to_string(),
                created_at: "0".to_string(),
            },
            ApiKeyConfig {
                id: "gemini-active".to_string(),
                provider: "gemini".to_string(),
                alias: None,
                status: "Active".to_string(),
                created_at: "0".to_string(),
            },
        ];

        assert_eq!(config.active_provider_key_count(), 1);
    }
}
