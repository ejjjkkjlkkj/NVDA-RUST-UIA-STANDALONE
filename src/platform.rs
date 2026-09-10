#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    WindowsUia,
    MacOsAccessibility,
    LinuxAtSpi2,
    AndroidAccessibilityService,
    IosAccessibilityClient,
    Unsupported,
}

impl BackendKind {
    pub const fn native_api(self) -> &'static str {
        match self {
            Self::WindowsUia => "Microsoft UI Automation (UIA)",
            Self::MacOsAccessibility => "macOS Accessibility (AXUIElement/AXObserver)",
            Self::LinuxAtSpi2 => "AT-SPI2 over D-Bus",
            Self::AndroidAccessibilityService => {
                "Android AccessibilityService / AccessibilityNodeInfo"
            }
            Self::IosAccessibilityClient => {
                "iOS UIAccessibility / Accessibility framework (app-scoped)"
            }
            Self::Unsupported => "unsupported accessibility backend",
        }
    }

    pub const fn is_system_screen_reader_capable(self) -> bool {
        matches!(
            self,
            Self::WindowsUia
                | Self::MacOsAccessibility
                | Self::LinuxAtSpi2
                | Self::AndroidAccessibilityService
        )
    }

    pub const fn is_native_backend_implemented(self) -> bool {
        matches!(self, Self::WindowsUia)
    }
}

pub const fn current_backend() -> BackendKind {
    if cfg!(target_os = "windows") {
        BackendKind::WindowsUia
    } else if cfg!(target_os = "macos") {
        BackendKind::MacOsAccessibility
    } else if cfg!(target_os = "android") {
        BackendKind::AndroidAccessibilityService
    } else if cfg!(target_os = "ios") {
        BackendKind::IosAccessibilityClient
    } else if cfg!(target_os = "linux") {
        BackendKind::LinuxAtSpi2
    } else {
        BackendKind::Unsupported
    }
}

pub trait AccessibilityBackend {
    type Error;

    fn name(&self) -> &'static str;
    fn run(&mut self, monitor_seconds: u64) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_api_names_are_stable() {
        assert_eq!(
            BackendKind::WindowsUia.native_api(),
            "Microsoft UI Automation (UIA)"
        );
        assert_eq!(
            BackendKind::MacOsAccessibility.native_api(),
            "macOS Accessibility (AXUIElement/AXObserver)"
        );
        assert_eq!(BackendKind::LinuxAtSpi2.native_api(), "AT-SPI2 over D-Bus");
        assert_eq!(
            BackendKind::AndroidAccessibilityService.native_api(),
            "Android AccessibilityService / AccessibilityNodeInfo"
        );
        assert_eq!(
            BackendKind::IosAccessibilityClient.native_api(),
            "iOS UIAccessibility / Accessibility framework (app-scoped)"
        );
    }

    #[test]
    fn system_screen_reader_capability_is_explicit() {
        assert!(BackendKind::WindowsUia.is_system_screen_reader_capable());
        assert!(BackendKind::MacOsAccessibility.is_system_screen_reader_capable());
        assert!(BackendKind::LinuxAtSpi2.is_system_screen_reader_capable());
        assert!(BackendKind::AndroidAccessibilityService.is_system_screen_reader_capable());
        assert!(!BackendKind::IosAccessibilityClient.is_system_screen_reader_capable());
    }

    #[test]
    fn windows_is_current_reference_backend() {
        assert!(BackendKind::WindowsUia.is_native_backend_implemented());
        assert!(!BackendKind::MacOsAccessibility.is_native_backend_implemented());
        assert!(!BackendKind::LinuxAtSpi2.is_native_backend_implemented());
        assert!(!BackendKind::AndroidAccessibilityService.is_native_backend_implemented());
        assert!(!BackendKind::IosAccessibilityClient.is_native_backend_implemented());
    }
}
