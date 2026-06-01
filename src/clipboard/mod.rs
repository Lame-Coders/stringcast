#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardSnapshot {
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    Unavailable,
    UnsupportedFormat,
}

pub trait ClipboardBackend {
    fn snapshot(&mut self) -> Result<ClipboardSnapshot, ClipboardError>;
    fn get_text(&mut self) -> Result<Option<String>, ClipboardError>;
    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError>;
    fn restore(&mut self, snapshot: &ClipboardSnapshot) -> Result<(), ClipboardError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryClipboard {
    text: Option<String>,
}

impl MemoryClipboard {
    pub fn new(text: Option<String>) -> Self {
        Self { text }
    }
}

impl ClipboardBackend for MemoryClipboard {
    fn snapshot(&mut self) -> Result<ClipboardSnapshot, ClipboardError> {
        Ok(ClipboardSnapshot {
            text: self.text.clone(),
        })
    }

    fn get_text(&mut self) -> Result<Option<String>, ClipboardError> {
        Ok(self.text.clone())
    }

    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.text = Some(text.to_string());
        Ok(())
    }

    fn restore(&mut self, snapshot: &ClipboardSnapshot) -> Result<(), ClipboardError> {
        self.text.clone_from(&snapshot.text);
        Ok(())
    }
}

pub struct ArboardClipboard {
    inner: arboard::Clipboard,
}

impl ArboardClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(Self {
            inner: arboard::Clipboard::new().map_err(|_| ClipboardError::Unavailable)?,
        })
    }
}

impl ClipboardBackend for ArboardClipboard {
    fn snapshot(&mut self) -> Result<ClipboardSnapshot, ClipboardError> {
        Ok(ClipboardSnapshot {
            text: self.get_text()?,
        })
    }

    fn get_text(&mut self) -> Result<Option<String>, ClipboardError> {
        match self.inner.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(_) => Err(ClipboardError::Unavailable),
        }
    }

    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.inner
            .set_text(text.to_string())
            .map_err(|_| ClipboardError::Unavailable)
    }

    fn restore(&mut self, snapshot: &ClipboardSnapshot) -> Result<(), ClipboardError> {
        match &snapshot.text {
            Some(text) => self.set_text(text),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_clipboard_snapshots_and_restores_text() {
        let mut clipboard = MemoryClipboard::new(Some("before".to_string()));
        let snapshot = clipboard.snapshot().unwrap();

        clipboard.set_text("after").unwrap();
        clipboard.restore(&snapshot).unwrap();

        assert_eq!(clipboard.get_text().unwrap(), Some("before".to_string()));
    }
}
