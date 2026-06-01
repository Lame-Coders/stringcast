use super::{InputEvent, InputHook, InputHookError, KeyShortcut, NavigationKey};
use rdev::{Event, EventType, Key};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ModifierState {
    shift: bool,
    control: bool,
    alt: bool,
    meta: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RdevEventNormalizer {
    modifiers: ModifierState,
}

#[derive(Debug, Clone, Default)]
pub struct RdevInputHook {
    normalizer: RdevEventNormalizer,
}

impl RdevInputHook {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InputHook for RdevInputHook {
    fn run<F>(&mut self, on_event: F) -> Result<(), InputHookError>
    where
        F: FnMut(InputEvent) + 'static,
    {
        let mut normalizer = self.normalizer.clone();
        let mut on_event = on_event;

        rdev::listen(move |event| {
            if let Some(input_event) = normalizer.normalize(event) {
                on_event(input_event);
            }
        })
        .map_err(|_| InputHookError::Unavailable)
    }
}

impl RdevEventNormalizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn normalize(&mut self, event: Event) -> Option<InputEvent> {
        match event.event_type {
            EventType::KeyPress(key) => self.key_press(key, event.name),
            EventType::KeyRelease(key) => {
                self.set_modifier(key, false);
                None
            }
            EventType::ButtonPress(_) => Some(InputEvent::MouseButton),
            _ => None,
        }
    }

    fn key_press(&mut self, key: Key, name: Option<String>) -> Option<InputEvent> {
        if self.set_modifier(key, true) {
            return None;
        }

        if let Some(shortcut) = self.shortcut_for(key) {
            return Some(InputEvent::Shortcut(shortcut));
        }

        match key {
            Key::Backspace => Some(InputEvent::Backspace),
            Key::Delete => Some(InputEvent::Delete),
            Key::Return => Some(InputEvent::Enter),
            Key::Escape => Some(InputEvent::Escape),
            Key::Tab => Some(InputEvent::Tab),
            Key::UpArrow => Some(InputEvent::Navigation(NavigationKey::ArrowUp)),
            Key::DownArrow => Some(InputEvent::Navigation(NavigationKey::ArrowDown)),
            Key::LeftArrow => Some(InputEvent::Navigation(NavigationKey::ArrowLeft)),
            Key::RightArrow => Some(InputEvent::Navigation(NavigationKey::ArrowRight)),
            Key::Home => Some(InputEvent::Navigation(NavigationKey::Home)),
            Key::End => Some(InputEvent::Navigation(NavigationKey::End)),
            Key::PageUp => Some(InputEvent::Navigation(NavigationKey::PageUp)),
            Key::PageDown => Some(InputEvent::Navigation(NavigationKey::PageDown)),
            Key::Space => Some(InputEvent::Text(" ".to_string())),
            _ => printable_text(name).map(InputEvent::Text),
        }
    }

    fn shortcut_for(&self, key: Key) -> Option<KeyShortcut> {
        if !self.primary_modifier_active() {
            return None;
        }

        match key {
            Key::KeyA => Some(KeyShortcut::SelectAll),
            Key::KeyC => Some(KeyShortcut::Copy),
            Key::KeyV => Some(KeyShortcut::Paste),
            Key::KeyX => Some(KeyShortcut::Cut),
            Key::KeyZ => Some(KeyShortcut::Undo),
            _ => Some(KeyShortcut::Other),
        }
    }

    fn primary_modifier_active(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.modifiers.meta
        } else {
            self.modifiers.control
        }
    }

    fn set_modifier(&mut self, key: Key, pressed: bool) -> bool {
        match key {
            Key::ShiftLeft | Key::ShiftRight => {
                self.modifiers.shift = pressed;
                true
            }
            Key::ControlLeft | Key::ControlRight => {
                self.modifiers.control = pressed;
                true
            }
            Key::Alt | Key::AltGr => {
                self.modifiers.alt = pressed;
                true
            }
            Key::MetaLeft | Key::MetaRight => {
                self.modifiers.meta = pressed;
                true
            }
            _ => false,
        }
    }
}

fn printable_text(name: Option<String>) -> Option<String> {
    let text = name?;
    if text.chars().all(|ch| !ch.is_control()) {
        Some(text)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdev::{Event, EventType};
    use std::time::SystemTime;

    fn event(event_type: EventType, name: Option<&str>) -> Event {
        Event {
            time: SystemTime::now(),
            name: name.map(str::to_string),
            event_type,
        }
    }

    #[test]
    fn printable_keypress_becomes_text_event() {
        let mut normalizer = RdevEventNormalizer::new();
        let input = normalizer.normalize(event(EventType::KeyPress(Key::KeyH), Some("h")));

        assert_eq!(input, Some(InputEvent::Text("h".to_string())));
    }

    #[test]
    fn navigation_key_maps_to_navigation_event() {
        let mut normalizer = RdevEventNormalizer::new();
        let input = normalizer.normalize(event(EventType::KeyPress(Key::LeftArrow), None));

        assert_eq!(
            input,
            Some(InputEvent::Navigation(NavigationKey::ArrowLeft))
        );
    }

    #[test]
    fn shortcut_clears_instead_of_appending_text() {
        let mut normalizer = RdevEventNormalizer::new();
        let modifier = if cfg!(target_os = "macos") {
            Key::MetaLeft
        } else {
            Key::ControlLeft
        };

        assert_eq!(
            normalizer.normalize(event(EventType::KeyPress(modifier), None)),
            None
        );
        let input = normalizer.normalize(event(EventType::KeyPress(Key::KeyV), Some("v")));

        assert_eq!(input, Some(InputEvent::Shortcut(KeyShortcut::Paste)));
    }

    #[test]
    fn mouse_button_maps_to_mouse_event() {
        let mut normalizer = RdevEventNormalizer::new();
        let input = normalizer.normalize(event(EventType::ButtonPress(rdev::Button::Left), None));

        assert_eq!(input, Some(InputEvent::MouseButton));
    }
}
