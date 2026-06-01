use crate::clipboard::{ClipboardBackend, ClipboardError};
use crate::input::{InputSimulationError, InputSimulator};
use crate::orchestrator::OperationSnapshot;
use std::thread;
use std::time::Duration;

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

    fn replace_current(
        &mut self,
        snapshot: &OperationSnapshot,
        current_text: &str,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        let _ = current_text;
        self.replace(snapshot, replacement_text)
    }
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

    fn replace_current(
        &mut self,
        snapshot: &OperationSnapshot,
        current_text: &str,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        let _ = current_text;
        self.replace(snapshot, replacement_text)
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardTextReplacer<C, I> {
    clipboard: C,
    input: I,
    select_all_wait: Duration,
    clipboard_restore_wait: Duration,
}

impl<C, I> ClipboardTextReplacer<C, I> {
    pub fn new(clipboard: C, input: I) -> Self {
        Self::with_delays(clipboard, input, Duration::ZERO, Duration::ZERO)
    }

    pub fn with_restore_wait(clipboard: C, input: I, clipboard_restore_wait: Duration) -> Self {
        Self::with_delays(clipboard, input, Duration::ZERO, clipboard_restore_wait)
    }

    pub fn with_delays(
        clipboard: C,
        input: I,
        select_all_wait: Duration,
        clipboard_restore_wait: Duration,
    ) -> Self {
        Self {
            clipboard,
            input,
            select_all_wait,
            clipboard_restore_wait,
        }
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
        snapshot: &OperationSnapshot,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        self.replace_current(snapshot, &snapshot.extracted_text, replacement_text)
    }

    fn replace_current(
        &mut self,
        _snapshot: &OperationSnapshot,
        _current_text: &str,
        replacement_text: &str,
    ) -> Result<(), ReplacementError> {
        let original_clipboard = self.clipboard.snapshot()?;
        self.clipboard.set_text(replacement_text)?;
        self.input.select_all()?;
        thread::sleep(self.select_all_wait);
        self.input.paste()?;
        thread::sleep(self.clipboard_restore_wait);
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
            replacement_target_text: "helo ?fix".to_string(),
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
