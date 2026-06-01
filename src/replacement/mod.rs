use crate::clipboard::{ClipboardBackend, ClipboardError};
use crate::input::{InputSimulationError, InputSimulator};
use crate::orchestrator::OperationSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplacementError {
    ForegroundChanged,
    PasteFailed,
    VerificationFailed,
    ClipboardUnavailable,
}

pub trait TextReplacer {
    fn replace(
        &mut self,
        snapshot: &OperationSnapshot,
        replacement_text: &str,
    ) -> Result<(), ReplacementError>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopTextReplacer {
    pub replacements: Vec<(OperationSnapshot, String)>,
}

impl TextReplacer for NoopTextReplacer {
    fn replace(
        &mut self,
        snapshot: &OperationSnapshot,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        self.replacements
            .push((snapshot.clone(), replacement_text.to_string()));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardTextReplacer<C, I> {
    clipboard: C,
    input: I,
}

impl<C, I> ClipboardTextReplacer<C, I> {
    pub fn new(clipboard: C, input: I) -> Self {
        Self { clipboard, input }
    }

    pub fn into_parts(self) -> (C, I) {
        (self.clipboard, self.input)
    }
}

impl<C, I> TextReplacer for ClipboardTextReplacer<C, I>
where
    C: ClipboardBackend,
    I: InputSimulator,
{
    fn replace(
        &mut self,
        _snapshot: &OperationSnapshot,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        let original_clipboard = self.clipboard.snapshot()?;
        self.clipboard.set_text(replacement_text)?;
        self.input.select_all()?;
        self.input.paste()?;
        self.clipboard.restore(&original_clipboard)?;
        Ok(())
    }
}

impl From<ClipboardError> for ReplacementError {
    fn from(_error: ClipboardError) -> Self {
        Self::ClipboardUnavailable
    }
}

impl From<InputSimulationError> for ReplacementError {
    fn from(_error: InputSimulationError) -> Self {
        Self::PasteFailed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::{ClipboardBackend, MemoryClipboard};
    use crate::input::{RecordedInputAction, RecordingInputSimulator};

    fn snapshot() -> OperationSnapshot {
        OperationSnapshot {
            operation_id: 1,
            app_id: "com.example.App".to_string(),
            window_id: None,
            extracted_text: "helo ?fix".to_string(),
            transform_input: "helo".to_string(),
            trigger_text: "?fix".to_string(),
        }
    }

    #[test]
    fn clipboard_replacer_pastes_and_restores_clipboard() {
        let clipboard = MemoryClipboard::new(Some("user clipboard".to_string()));
        let input = RecordingInputSimulator::default();
        let mut replacer = ClipboardTextReplacer::new(clipboard, input);

        replacer.replace(&snapshot(), "hello").unwrap();
        let (mut clipboard, input) = replacer.into_parts();

        assert_eq!(
            clipboard.get_text().unwrap(),
            Some("user clipboard".to_string())
        );
        assert_eq!(
            input.actions,
            vec![RecordedInputAction::SelectAll, RecordedInputAction::Paste]
        );
    }
}
