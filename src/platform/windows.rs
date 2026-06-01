use super::{ForegroundApp, ForegroundAppProvider, PlatformContextError};

#[derive(Debug, Clone, Default)]
pub struct WindowsForegroundAppProvider;

impl WindowsForegroundAppProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ForegroundAppProvider for WindowsForegroundAppProvider {
    fn foreground_app(&mut self) -> Result<ForegroundApp, PlatformContextError> {
        Err(PlatformContextError::Unavailable)
    }
}
