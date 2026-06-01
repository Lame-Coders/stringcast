use crate::storage::CommandsConfig;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInCommand {
    Fix,
    Improve,
    Shorten,
    Expand,
    Formal,
    Casual,
    Emoji,
    Reply,
    Bullets,
    Summarize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicCommand {
    Translate { lang_code: String },
    Ask { question: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    BuiltIn(BuiltInCommand),
    Custom,
    Dynamic(DynamicCommand),
    Undo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDefinition {
    pub trigger: String,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub case_sensitive: bool,
    pub raw_output: bool,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCommand {
    pub trigger: String,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub case_sensitive: bool,
    pub raw_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomCommandError {
    TriggerMustStartWithQuestionMark,
    TriggerLength,
    TriggerCharacters,
    ReservedTrigger,
    NameLength,
    MissingTextPlaceholder,
    PromptLength,
}

#[derive(Debug, Clone)]
pub struct CommandRegistry {
    custom: Vec<CommandDefinition>,
    disabled_builtins: HashSet<String>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            custom: Vec::new(),
            disabled_builtins: HashSet::new(),
        }
    }

    pub fn with_custom_commands(commands: Vec<CustomCommand>) -> Result<Self, CustomCommandError> {
        let mut registry = Self::new();
        for command in commands {
            registry.add_custom(command)?;
        }
        Ok(registry)
    }

    pub fn from_config(config: &CommandsConfig) -> Result<Self, CustomCommandError> {
        let mut registry = Self::new();
        for trigger in &config.disabled_builtins {
            registry.disable_builtin(trigger.clone());
        }
        for command in &config.custom {
            registry.add_custom(CustomCommand {
                trigger: command.trigger.clone(),
                name: command.name.clone(),
                prompt: command.prompt.clone(),
                enabled: command.enabled,
                case_sensitive: command.case_sensitive,
                raw_output: command.raw_output,
            })?;
        }
        Ok(registry)
    }

    pub fn add_custom(&mut self, command: CustomCommand) -> Result<(), CustomCommandError> {
        command.validate()?;
        self.custom.push(command.into_definition());
        self.sort_custom();
        Ok(())
    }

    pub fn disable_builtin(&mut self, trigger: impl Into<String>) {
        self.disabled_builtins.insert(trigger.into());
    }

    pub fn resolve_static(&self, trigger: &str) -> Option<CommandDefinition> {
        if let Some(command) = self
            .custom
            .iter()
            .find(|command| command.enabled && command.trigger_matches(trigger))
        {
            return Some(command.clone());
        }

        if trigger == "?undo" {
            return Some(CommandDefinition {
                trigger: "?undo".to_string(),
                name: "Undo".to_string(),
                prompt: String::new(),
                enabled: true,
                case_sensitive: true,
                raw_output: true,
                kind: CommandKind::Undo,
            });
        }

        if self.disabled_builtins.contains(trigger) {
            return None;
        }

        builtin_definitions()
            .into_iter()
            .find(|command| command.trigger == trigger && command.enabled)
    }

    pub fn static_triggers_longest_first(&self) -> Vec<String> {
        let mut triggers: Vec<String> = self
            .custom
            .iter()
            .filter(|command| command.enabled)
            .map(|command| command.trigger.clone())
            .collect();

        triggers.push("?undo".to_string());
        triggers.extend(
            builtin_definitions()
                .into_iter()
                .filter(|command| !self.disabled_builtins.contains(&command.trigger))
                .map(|command| command.trigger),
        );

        triggers.sort_by_key(|trigger| std::cmp::Reverse(trigger.len()));
        triggers.dedup();
        triggers
    }

    fn sort_custom(&mut self) {
        self.custom
            .sort_by_key(|command| std::cmp::Reverse(command.trigger.len()));
    }
}

impl CustomCommand {
    pub fn validate(&self) -> Result<(), CustomCommandError> {
        if !self.trigger.starts_with('?') {
            return Err(CustomCommandError::TriggerMustStartWithQuestionMark);
        }

        if !(2..=64).contains(&self.trigger.chars().count()) {
            return Err(CustomCommandError::TriggerLength);
        }

        if !self.trigger.chars().all(|ch| {
            ch == '?' || ch == ':' || ch == '-' || ch == '_' || ch.is_ascii_alphanumeric()
        }) {
            return Err(CustomCommandError::TriggerCharacters);
        }

        if self.trigger == "?undo"
            || self.trigger.starts_with("?translate:")
            || self.trigger.starts_with("?ask:")
        {
            return Err(CustomCommandError::ReservedTrigger);
        }

        if !(1..=64).contains(&self.name.chars().count()) {
            return Err(CustomCommandError::NameLength);
        }

        if !self.prompt.contains("{text}") {
            return Err(CustomCommandError::MissingTextPlaceholder);
        }

        if self.prompt.chars().count() > 4096 {
            return Err(CustomCommandError::PromptLength);
        }

        Ok(())
    }

    fn into_definition(self) -> CommandDefinition {
        CommandDefinition {
            trigger: self.trigger,
            name: self.name,
            prompt: self.prompt,
            enabled: self.enabled,
            case_sensitive: self.case_sensitive,
            raw_output: self.raw_output,
            kind: CommandKind::Custom,
        }
    }
}

impl CommandDefinition {
    fn trigger_matches(&self, trigger: &str) -> bool {
        if self.case_sensitive {
            self.trigger == trigger
        } else {
            self.trigger.eq_ignore_ascii_case(trigger)
        }
    }
}

fn builtin_definitions() -> Vec<CommandDefinition> {
    vec![
        builtin("?fix", "Fix Grammar", BuiltInCommand::Fix, "Fix all grammar, spelling, and punctuation errors in the following text. Return ONLY the corrected text, nothing else:\n\n{text}"),
        builtin("?improve", "Improve", BuiltInCommand::Improve, "Improve the clarity, flow, and readability of the following text while preserving its meaning and tone. Return ONLY the improved text, nothing else:\n\n{text}"),
        builtin("?shorten", "Shorten", BuiltInCommand::Shorten, "Shorten the following text to its most essential meaning without losing key information. Return ONLY the shortened text, nothing else:\n\n{text}"),
        builtin("?expand", "Expand", BuiltInCommand::Expand, "Expand the following text with relevant detail, context, and supporting explanation. Return ONLY the expanded text, nothing else:\n\n{text}"),
        builtin("?formal", "Make Formal", BuiltInCommand::Formal, "Rewrite the following text in a professional, formal tone suitable for business communication. Return ONLY the rewritten text, nothing else:\n\n{text}"),
        builtin("?casual", "Make Casual", BuiltInCommand::Casual, "Rewrite the following text in a friendly, casual, conversational tone. Return ONLY the rewritten text, nothing else:\n\n{text}"),
        builtin("?emoji", "Add Emojis", BuiltInCommand::Emoji, "Add relevant and tasteful emojis to the following text to make it more expressive. Return ONLY the text with emojis added, nothing else:\n\n{text}"),
        builtin("?reply", "Generate Reply", BuiltInCommand::Reply, "Generate a natural, contextually appropriate reply to the following message. Return ONLY the reply text, nothing else:\n\n{text}"),
        builtin("?bullets", "Bullet Points", BuiltInCommand::Bullets, "Convert the following text into a concise, well-structured bullet-point list. Return ONLY the bullet points, nothing else:\n\n{text}"),
        builtin("?summarize", "Summarize", BuiltInCommand::Summarize, "Write a concise summary of the following text in 1-3 sentences. Return ONLY the summary, nothing else:\n\n{text}"),
    ]
}

fn builtin(trigger: &str, name: &str, command: BuiltInCommand, prompt: &str) -> CommandDefinition {
    CommandDefinition {
        trigger: trigger.to_string(),
        name: name.to_string(),
        prompt: prompt.to_string(),
        enabled: true,
        case_sensitive: true,
        raw_output: false,
        kind: CommandKind::BuiltIn(command),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_command_can_override_builtin_trigger() {
        let mut registry = CommandRegistry::new();
        registry
            .add_custom(CustomCommand {
                trigger: "?fix".to_string(),
                name: "Custom Fix".to_string(),
                prompt: "Rewrite this exactly: {text}".to_string(),
                enabled: true,
                case_sensitive: true,
                raw_output: false,
            })
            .unwrap();

        let command = registry.resolve_static("?fix").unwrap();

        assert_eq!(command.kind, CommandKind::Custom);
        assert_eq!(command.name, "Custom Fix");
    }

    #[test]
    fn custom_command_rejects_reserved_dynamic_prefixes() {
        let command = CustomCommand {
            trigger: "?ask:team".to_string(),
            name: "Ask Team".to_string(),
            prompt: "Answer: {text}".to_string(),
            enabled: true,
            case_sensitive: true,
            raw_output: false,
        };

        assert_eq!(command.validate(), Err(CustomCommandError::ReservedTrigger));
    }

    #[test]
    fn registry_builds_from_config() {
        let registry = CommandRegistry::from_config(&CommandsConfig {
            disabled_builtins: vec!["?emoji".to_string()],
            custom: vec![crate::storage::CommandConfig {
                trigger: "?ship".to_string(),
                name: "Ship It".to_string(),
                prompt: "Make this concise: {text}".to_string(),
                enabled: true,
                case_sensitive: true,
                raw_output: false,
                created_at: "2026-05-31T00:00:00Z".to_string(),
            }],
        })
        .unwrap();

        assert!(registry.resolve_static("?emoji").is_none());
        assert_eq!(
            registry.resolve_static("?ship").unwrap().kind,
            CommandKind::Custom
        );
    }
}
