#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Text(String),
    Backspace,
    Delete,
    Enter,
    Escape,
    Tab,
    Navigation(NavigationKey),
    MouseButton,
    Shortcut(KeyShortcut),
    FocusChanged,
    SleepOrLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyShortcut {
    SelectAll,
    Copy,
    Paste,
    Cut,
    Undo,
    Other,
}
