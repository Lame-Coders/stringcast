pub const DEFAULT_MAX_BUFFER_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferEvent {
    Text(String),
    Backspace,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeystrokeBuffer {
    text: String,
    max_bytes: usize,
}

impl Default for KeystrokeBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_BUFFER_BYTES)
    }
}

impl KeystrokeBuffer {
    pub fn new(max_bytes: usize) -> Self {
        assert!(max_bytes > 0, "buffer must retain at least one byte");
        Self {
            text: String::new(),
            max_bytes,
        }
    }

    pub fn apply(&mut self, event: BufferEvent) {
        match event {
            BufferEvent::Text(text) => self.append(&text),
            BufferEvent::Backspace => self.backspace(),
            BufferEvent::Clear => self.clear(),
        }
    }

    pub fn append(&mut self, text: &str) {
        self.text.push_str(text);
        self.trim_to_limit();
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn trim_to_limit(&mut self) {
        if self.text.len() <= self.max_bytes {
            return;
        }

        let mut start = self.text.len() - self.max_bytes;
        while !self.text.is_char_boundary(start) {
            start += 1;
        }
        self.text.drain(..start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backspace_removes_last_unicode_scalar() {
        let mut buffer = KeystrokeBuffer::default();
        buffer.append("abé");

        buffer.backspace();

        assert_eq!(buffer.as_str(), "ab");
    }

    #[test]
    fn overflow_keeps_tail_without_splitting_utf8() {
        let mut buffer = KeystrokeBuffer::new(5);
        buffer.append("abcédef");

        assert_eq!(buffer.as_str(), "édef");
    }

    #[test]
    fn clear_removes_all_text() {
        let mut buffer = KeystrokeBuffer::default();
        buffer.append("hello");

        buffer.apply(BufferEvent::Clear);

        assert!(buffer.is_empty());
    }
}
