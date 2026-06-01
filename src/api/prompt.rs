use super::{ApiClient, ApiClientError, HttpTransport, KeyMaterialStore};
use crate::commands::{CommandDefinition, CommandKind, DynamicCommand};
use crate::pipeline::{TextTransformer, TransformError};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPrompt {
    pub system_prompt: String,
    pub user_text: String,
    pub raw_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptRenderError {
    UndoDoesNotUseApi,
    MissingTextPlaceholder,
}

pub fn render_prompt(
    command: &CommandDefinition,
    input: &str,
) -> Result<RenderedPrompt, PromptRenderError> {
    match &command.kind {
        CommandKind::Undo => Err(PromptRenderError::UndoDoesNotUseApi),
        CommandKind::Dynamic(DynamicCommand::Translate { lang_code }) => {
            let language_name = language_name(lang_code).unwrap_or(lang_code);
            Ok(RenderedPrompt {
                system_prompt: format!(
                    "Translate the following text to {language_name} (language code: {lang_code}). Return ONLY the translated text, nothing else."
                ),
                user_text: input.to_string(),
                raw_output: command.raw_output,
            })
        }
        CommandKind::Dynamic(DynamicCommand::Ask { question }) => Ok(RenderedPrompt {
            system_prompt: format!(
                "Given the following text, answer this instruction: {question}. Return ONLY your response, without any preamble or commentary."
            ),
            user_text: input.to_string(),
            raw_output: command.raw_output,
        }),
        CommandKind::BuiltIn(_) | CommandKind::Custom => {
            if !command.prompt.contains("{text}") {
                return Err(PromptRenderError::MissingTextPlaceholder);
            }

            Ok(RenderedPrompt {
                system_prompt: command
                    .prompt
                    .replace("{text}", "")
                    .trim()
                    .trim_end_matches(':')
                    .trim()
                    .to_string(),
                user_text: input.to_string(),
                raw_output: command.raw_output,
            })
        }
    }
}

#[derive(Debug)]
pub struct ApiTextTransformer<T, K> {
    client: ApiClient<T, K>,
}

impl<T, K> ApiTextTransformer<T, K> {
    pub fn new(client: ApiClient<T, K>) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &ApiClient<T, K> {
        &self.client
    }
}

impl<T, K> TextTransformer for ApiTextTransformer<T, K>
where
    T: HttpTransport,
    K: KeyMaterialStore,
{
    fn transform(
        &mut self,
        command: &CommandDefinition,
        input: &str,
    ) -> Result<String, TransformError> {
        let rendered =
            render_prompt(command, input).map_err(|_| TransformError::ProviderRejected)?;
        self.client
            .transform(
                &rendered.system_prompt,
                &rendered.user_text,
                rendered.raw_output,
                Instant::now(),
            )
            .map_err(map_api_error)
    }
}

fn map_api_error(error: ApiClientError) -> TransformError {
    match error {
        ApiClientError::NoKeysAvailable | ApiClientError::MissingKeyMaterial(_) => {
            TransformError::ApiUnavailable
        }
        ApiClientError::Provider(super::ProviderError::EmptyResponse) => {
            TransformError::EmptyOutput
        }
        ApiClientError::Transport(_) => TransformError::ApiUnavailable,
        ApiClientError::BadRequest
        | ApiClientError::Provider(_)
        | ApiClientError::RetryExhausted
        | ApiClientError::UnexpectedStatus(_) => TransformError::ProviderRejected,
    }
}

fn language_name(code: &str) -> Option<&'static str> {
    match code {
        "en" => Some("English"),
        "es" => Some("Spanish"),
        "fr" => Some("French"),
        "de" => Some("German"),
        "hi" => Some("Hindi"),
        "ja" => Some("Japanese"),
        "ko" => Some("Korean"),
        "pt" => Some("Portuguese"),
        "zh" | "zh-Hans" => Some("Simplified Chinese"),
        "zh-Hant" => Some("Traditional Chinese"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{BuiltInCommand, CommandKind, DynamicCommand};

    #[test]
    fn renders_builtin_prompt_as_system_plus_user_text() {
        let command = CommandDefinition {
            trigger: "?fix".to_string(),
            name: "Fix".to_string(),
            prompt: "Fix this. Return ONLY text:\n\n{text}".to_string(),
            enabled: true,
            case_sensitive: true,
            raw_output: false,
            kind: CommandKind::BuiltIn(BuiltInCommand::Fix),
        };

        let rendered = render_prompt(&command, "helo").unwrap();

        assert_eq!(rendered.system_prompt, "Fix this. Return ONLY text");
        assert_eq!(rendered.user_text, "helo");
    }

    #[test]
    fn renders_dynamic_translate_prompt() {
        let command = CommandDefinition {
            trigger: "?translate:es".to_string(),
            name: "Translate".to_string(),
            prompt: String::new(),
            enabled: true,
            case_sensitive: true,
            raw_output: false,
            kind: CommandKind::Dynamic(DynamicCommand::Translate {
                lang_code: "es".to_string(),
            }),
        };

        let rendered = render_prompt(&command, "hello").unwrap();

        assert!(rendered.system_prompt.contains("Spanish"));
        assert_eq!(rendered.user_text, "hello");
    }
}
