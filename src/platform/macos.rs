use super::{
    ForegroundApp, ForegroundAppProvider, PermissionChecker, PermissionReport, PermissionStatus,
    PlatformContextError,
};
use std::process::Command;
use std::{ffi::c_void, ptr};

#[derive(Debug, Clone, Default)]
pub struct MacOsForegroundAppProvider;

#[derive(Debug, Clone, Copy, Default)]
pub struct MacOsPermissionChecker;

impl MacOsForegroundAppProvider {
    pub fn new() -> Self {
        Self
    }
}

impl MacOsPermissionChecker {
    pub fn new() -> Self {
        Self
    }
}

impl ForegroundAppProvider for MacOsForegroundAppProvider {
    fn foreground_app(&mut self) -> Result<ForegroundApp, PlatformContextError> {
        let output = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set bundleId to bundle identifier of frontApp
                    set appName to name of frontApp
                    return bundleId & linefeed & appName
                end tell"#,
            ])
            .output()
            .map_err(|_| PlatformContextError::Unavailable)?;

        if !output.status.success() {
            return Err(PlatformContextError::CommandFailed);
        }

        let stdout =
            String::from_utf8(output.stdout).map_err(|_| PlatformContextError::InvalidOutput)?;
        let (bundle_id, display_name) = parse_frontmost_app_output(&stdout)?;

        Ok(ForegroundApp {
            app_id: bundle_id,
            window_id: None,
            display_name,
            secure_input: secure_event_input_enabled(),
            elevated: false,
        })
    }
}

impl PermissionChecker for MacOsPermissionChecker {
    fn permission_report(&self) -> PermissionReport {
        PermissionReport {
            accessibility: if accessibility_trusted() {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Missing
            },
            // macOS does not expose a stable public yes/no API for Input Monitoring.
            // rdev will still fail at hook registration if this is missing.
            input_monitoring: PermissionStatus::Unknown,
        }
    }
}

fn parse_frontmost_app_output(
    stdout: &str,
) -> Result<(String, Option<String>), PlatformContextError> {
    let mut lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let bundle_id = lines
        .next()
        .ok_or(PlatformContextError::InvalidOutput)?
        .to_string();
    let display_name = lines.next().map(str::to_string);

    if bundle_id.is_empty() {
        return Err(PlatformContextError::InvalidOutput);
    }

    Ok((bundle_id, display_name))
}

fn secure_event_input_enabled() -> bool {
    unsafe { IsSecureEventInputEnabled() }
}

fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_accessibility_permission() -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        );

        let trusted = AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
        trusted
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
    fn AXIsProcessTrusted() -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: *const c_void;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    fn CFRelease(cf: *const c_void);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundle_id_and_display_name() {
        let (bundle_id, display_name) =
            parse_frontmost_app_output("com.apple.TextEdit\nTextEdit\n").unwrap();

        assert_eq!(bundle_id, "com.apple.TextEdit");
        assert_eq!(display_name.as_deref(), Some("TextEdit"));
    }

    #[test]
    fn rejects_empty_output() {
        assert_eq!(
            parse_frontmost_app_output(""),
            Err(PlatformContextError::InvalidOutput)
        );
    }
}
