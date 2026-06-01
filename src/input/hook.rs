use super::InputEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputHookError {
    Unavailable,
    PermissionDenied,
}

pub trait InputHook {
    fn run<F>(&mut self, on_event: F) -> Result<(), InputHookError>
    where
        F: FnMut(InputEvent) + 'static;
}
