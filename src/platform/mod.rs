use crate::storage::ExclusionsConfig;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{MacOsForegroundAppProvider, MacOsPermissionChecker};

#[cfg(target_os = "windows")]
pub use windows::WindowsForegroundAppProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundApp {
    pub app_id: String,
    pub window_id: Option<String>,
    pub display_name: Option<String>,
    pub secure_input: bool,
    pub elevated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformContextError {
    Unavailable,
    CommandFailed,
    InvalidOutput,
}

pub trait ForegroundAppProvider {
    fn foreground_app(&mut self) -> Result<ForegroundApp, PlatformContextError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionReport {
    pub accessibility: PermissionStatus,
    pub input_monitoring: PermissionStatus,
}

impl PermissionReport {
    pub fn is_ready(&self) -> bool {
        self.accessibility == PermissionStatus::Granted
            && self.input_monitoring != PermissionStatus::Missing
    }

    pub fn startup_error_message(&self) -> Option<String> {
        if self.is_ready() {
            return None;
        }

        Some(format!(
            "required permissions are missing. Accessibility: {:?}, Input Monitoring: {:?}. On macOS, grant Stringcast permissions in System Settings > Privacy & Security > Accessibility and Input Monitoring, then restart Stringcast.",
            self.accessibility, self.input_monitoring
        ))
    }
}

pub trait PermissionChecker {
    fn permission_report(&self) -> PermissionReport;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopPermissionChecker;

impl PermissionChecker for NoopPermissionChecker {
    fn permission_report(&self) -> PermissionReport {
        PermissionReport {
            accessibility: PermissionStatus::Granted,
            input_monitoring: PermissionStatus::Granted,
        }
    }
}

#[cfg(target_os = "macos")]
pub type SystemPermissionChecker = MacOsPermissionChecker;

#[cfg(not(target_os = "macos"))]
pub type SystemPermissionChecker = NoopPermissionChecker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticForegroundAppProvider {
    app: ForegroundApp,
}

impl StaticForegroundAppProvider {
    pub fn new(app: ForegroundApp) -> Self {
        Self { app }
    }
}

impl ForegroundAppProvider for StaticForegroundAppProvider {
    fn foreground_app(&mut self) -> Result<ForegroundApp, PlatformContextError> {
        Ok(self.app.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusionMatcher {
    entries: Vec<String>,
}

impl ExclusionMatcher {
    pub fn new(entries: impl IntoIterator<Item = String>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .collect(),
        }
    }

    pub fn from_config(config: &ExclusionsConfig) -> Self {
        Self::new(
            default_exclusions()
                .into_iter()
                .chain(config.apps.clone())
                .chain(config.known_problematic_apps.clone()),
        )
    }

    pub fn is_excluded(&self, app_id: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(app_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationGateDecision {
    Allow,
    BlockedDisabled,
    BlockedExcluded,
    BlockedSecureInput,
    BlockedElevated,
    ContextUnavailable,
}

#[derive(Debug, Clone)]
pub struct OperationGate {
    enabled: bool,
    exclusions: ExclusionMatcher,
    allow_elevated: bool,
}

impl OperationGate {
    pub fn new(enabled: bool, exclusions: ExclusionMatcher) -> Self {
        Self {
            enabled,
            exclusions,
            allow_elevated: false,
        }
    }

    pub fn with_allow_elevated(mut self, allow_elevated: bool) -> Self {
        self.allow_elevated = allow_elevated;
        self
    }

    pub fn evaluate(&self, app: &ForegroundApp) -> OperationGateDecision {
        if !self.enabled {
            return OperationGateDecision::BlockedDisabled;
        }
        if self.exclusions.is_excluded(&app.app_id) {
            return OperationGateDecision::BlockedExcluded;
        }
        if app.secure_input {
            return OperationGateDecision::BlockedSecureInput;
        }
        if app.elevated && !self.allow_elevated {
            return OperationGateDecision::BlockedElevated;
        }

        OperationGateDecision::Allow
    }
}

pub fn default_exclusions() -> Vec<String> {
    vec![
        "com.apple.keychainaccess".to_string(),
        "com.1password.1password".to_string(),
        "com.agilebits.onepassword7".to_string(),
        "com.bitwarden.desktop".to_string(),
        "com.lastpass.lastpass-mac".to_string(),
        "com.dashlane.Dashlane".to_string(),
        "1Password.exe".to_string(),
        "KeePass.exe".to_string(),
        "KeePassXC.exe".to_string(),
        "Bitwarden.exe".to_string(),
        "LastPass.exe".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(app_id: &str) -> ForegroundApp {
        ForegroundApp {
            app_id: app_id.to_string(),
            window_id: Some("window".to_string()),
            display_name: None,
            secure_input: false,
            elevated: false,
        }
    }

    #[test]
    fn default_exclusions_block_password_managers() {
        let matcher = ExclusionMatcher::from_config(&ExclusionsConfig::default());

        assert!(matcher.is_excluded("1Password.exe"));
        assert!(matcher.is_excluded("com.1password.1password"));
    }

    #[test]
    fn custom_exclusions_are_case_insensitive() {
        let matcher = ExclusionMatcher::new(["MyApp.exe".to_string()]);

        assert!(matcher.is_excluded("myapp.exe"));
    }

    #[test]
    fn gate_blocks_secure_and_elevated_contexts() {
        let gate = OperationGate::new(true, ExclusionMatcher::new(Vec::new()));

        assert_eq!(
            gate.evaluate(&ForegroundApp {
                secure_input: true,
                ..app("Mail.app")
            }),
            OperationGateDecision::BlockedSecureInput
        );
        assert_eq!(
            gate.evaluate(&ForegroundApp {
                elevated: true,
                ..app("Admin.exe")
            }),
            OperationGateDecision::BlockedElevated
        );
    }

    #[test]
    fn permission_report_formats_actionable_startup_error() {
        let report = PermissionReport {
            accessibility: PermissionStatus::Missing,
            input_monitoring: PermissionStatus::Unknown,
        };
        let message = report.startup_error_message().unwrap();

        assert!(message.contains("Accessibility"));
        assert!(message.contains("Input Monitoring"));
        assert!(message.contains("System Settings"));
    }

    #[test]
    fn granted_permission_report_is_ready() {
        let report = PermissionReport {
            accessibility: PermissionStatus::Granted,
            input_monitoring: PermissionStatus::Granted,
        };

        assert!(report.is_ready());
        assert!(report.startup_error_message().is_none());
    }
}
