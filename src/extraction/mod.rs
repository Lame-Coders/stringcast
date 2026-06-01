use crate::clipboard::{ClipboardBackend, ClipboardError};
use crate::detection::TriggerMatch;
use crate::input::{InputSimulationError, InputSimulator};
use crate::orchestrator::OperationSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionContext {
    pub operation_id: u64,
    pub app_id: String,
    pub window_id: Option<String>,
    pub trigger_match: TriggerMatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionError {
    ClipboardUnavailable,
    CopyFailed,
    TriggerMissingFromSnapshot,
    AppBlocked,
}

pub trait TextExtractor {
    fn extract(&mut self, context: ExtractionContext)
        -> Result<OperationSnapshot, ExtractionError>;
}

#[derive(Debug, Clone, Default)]
pub struct BufferTextExtractor;

impl TextExtractor for BufferTextExtractor {
    fn extract(
        &mut self,
        context: ExtractionContext,
    ) -> Result<OperationSnapshot, ExtractionError> {
        Ok(OperationSnapshot {
            operation_id: context.operation_id,
            app_id: context.app_id,
            window_id: context.window_id,
            extracted_text: format!(
                "{} {}",
                context.trigger_match.transform_input, context.trigger_match.trigger_text
            ),
            transform_input: context.trigger_match.transform_input,
            trigger_text: context.trigger_match.trigger_text,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardTextExtractor<C, I> {
    clipboard: C,
    input: I,
}

impl<C, I> ClipboardTextExtractor<C, I> {
    pub fn new(clipboard: C, input: I) -> Self {
        Self { clipboard, input }
    }

    pub fn into_parts(self) -> (C, I) {
        (self.clipboard, self.input)
    }
}

impl<C, I> TextExtractor for ClipboardTextExtractor<C, I>
where
    C: ClipboardBackend,
    I: InputSimulator,
{
    fn extract(
        &mut self,
        context: ExtractionContext,
    ) -> Result<OperationSnapshot, ExtractionError> {
        let original_clipboard = self.clipboard.snapshot()?;

        self.input.select_all()?;
        self.input.copy()?;

        let copied_text = self
            .clipboard
            .get_text()?
            .ok_or(ExtractionError::TriggerMissingFromSnapshot)?;

        self.clipboard.restore(&original_clipboard)?;

        let transform_input =
            transform_input_from_snapshot(&copied_text, &context.trigger_match.trigger_text)?;

        Ok(OperationSnapshot {
            operation_id: context.operation_id,
            app_id: context.app_id,
            window_id: context.window_id,
            extracted_text: copied_text,
            transform_input,
            trigger_text: context.trigger_match.trigger_text,
        })
    }
}

fn transform_input_from_snapshot(
    copied_text: &str,
    trigger_text: &str,
) -> Result<String, ExtractionError> {
    let trimmed = copied_text.trim_end();
    if !trimmed.ends_with(trigger_text) {
        return Err(ExtractionError::TriggerMissingFromSnapshot);
    }

    let input_end = trimmed.len() - trigger_text.len();
    let transform_input = trimmed[..input_end].trim_end().to_string();
    if transform_input.is_empty() {
        return Err(ExtractionError::TriggerMissingFromSnapshot);
    }

    Ok(transform_input)
}

impl From<ClipboardError> for ExtractionError {
    fn from(_error: ClipboardError) -> Self {
        Self::ClipboardUnavailable
    }
}

impl From<InputSimulationError> for ExtractionError {
    fn from(_error: InputSimulationError) -> Self {
        Self::CopyFailed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::MemoryClipboard;
    use crate::commands::{BuiltInCommand, CommandDefinition, CommandKind};
    use crate::input::{RecordedInputAction, RecordingInputSimulator};

    fn context() -> ExtractionContext {
        ExtractionContext {
            operation_id: 7,
            app_id: "com.example.App".to_string(),
            window_id: Some("window".to_string()),
            trigger_match: TriggerMatch {
                trigger_text: "?fix".to_string(),
                transform_input: "buffer fallback".to_string(),
                command: CommandDefinition {
                    trigger: "?fix".to_string(),
                    name: "Fix".to_string(),
                    prompt: "Fix: {text}".to_string(),
                    enabled: true,
                    case_sensitive: true,
                    raw_output: false,
                    kind: CommandKind::BuiltIn(BuiltInCommand::Fix),
                },
            },
        }
    }

    #[test]
    fn clipboard_extractor_uses_copied_field_snapshot() {
        let clipboard = MemoryClipboard::new(Some("actual field text ?fix".to_string()));
        let input = RecordingInputSimulator::default();
        let mut extractor = ClipboardTextExtractor::new(clipboard, input);

        let snapshot = extractor.extract(context()).unwrap();
        let (_, input) = extractor.into_parts();

        assert_eq!(snapshot.extracted_text, "actual field text ?fix");
        assert_eq!(snapshot.transform_input, "actual field text");
        assert_eq!(
            input.actions,
            vec![RecordedInputAction::SelectAll, RecordedInputAction::Copy]
        );
    }

    #[test]
    fn clipboard_extractor_rejects_snapshot_without_trigger() {
        let clipboard = MemoryClipboard::new(Some("actual field text".to_string()));
        let input = RecordingInputSimulator::default();
        let mut extractor = ClipboardTextExtractor::new(clipboard, input);

        let result = extractor.extract(context());

        assert_eq!(result, Err(ExtractionError::TriggerMissingFromSnapshot));
    }
}
