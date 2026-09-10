use std::fmt;

pub mod platform;
pub mod presentation;
pub mod semantic;
pub mod uia;

pub const DEFAULT_MONITOR_SECONDS: u64 = 15;

pub fn parse_monitor_seconds(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MONITOR_SECONDS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityEventKind {
    Focus,
    TextChanged,
    TextSelectionChanged,
}

impl AccessibilityEventKind {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::TextChanged => "text_changed",
            Self::TextSelectionChanged => "text_selection_changed",
        }
    }
}

impl fmt::Display for AccessibilityEventKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Focus => "FOCUS",
            Self::TextChanged => "TEXT_CHANGED/UIA_20015",
            Self::TextSelectionChanged => "TEXT_SELECTION_CHANGED/UIA_20014",
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementSnapshot {
    pub process_id: i32,
    pub framework: String,
    pub class_name: String,
    /// Platform-neutral role derived from the stable UIA ControlTypeId.
    pub role: String,
    /// Provider/localized role retained only as diagnostic native metadata.
    pub native_role: String,
    pub control_type_id: i32,
    pub name: String,
    pub automation_id: String,
}

pub fn format_event_line(
    kind: AccessibilityEventKind,
    sequence: u64,
    element: &ElementSnapshot,
) -> String {
    format!(
        "{kind} #{sequence} | PID={} | Framework={} | Class={} | Role={} | Name={} | AutomationId={} | ControlTypeId={} | NativeRole={}",
        element.process_id,
        element.framework,
        element.class_name,
        element.role,
        element.name,
        element.automation_id,
        element.control_type_id,
        element.native_role
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_value_uses_default() {
        assert_eq!(parse_monitor_seconds(None), DEFAULT_MONITOR_SECONDS);
    }

    #[test]
    fn invalid_value_uses_default() {
        assert_eq!(
            parse_monitor_seconds(Some("invalid")),
            DEFAULT_MONITOR_SECONDS
        );
    }

    #[test]
    fn valid_value_is_preserved() {
        assert_eq!(parse_monitor_seconds(Some("30")), 30);
    }

    #[test]
    fn zero_is_allowed_for_noninteractive_smoke_runs() {
        assert_eq!(parse_monitor_seconds(Some("0")), 0);
    }

    #[test]
    fn canonical_event_names_are_platform_neutral() {
        assert_eq!(AccessibilityEventKind::Focus.canonical_name(), "focus");
        assert_eq!(
            AccessibilityEventKind::TextChanged.canonical_name(),
            "text_changed"
        );
        assert_eq!(
            AccessibilityEventKind::TextSelectionChanged.canonical_name(),
            "text_selection_changed"
        );
    }

    #[test]
    fn event_kind_keeps_windows_compatibility_label() {
        assert_eq!(AccessibilityEventKind::Focus.to_string(), "FOCUS");
        assert_eq!(
            AccessibilityEventKind::TextChanged.to_string(),
            "TEXT_CHANGED/UIA_20015"
        );
        assert_eq!(
            AccessibilityEventKind::TextSelectionChanged.to_string(),
            "TEXT_SELECTION_CHANGED/UIA_20014"
        );
    }

    #[test]
    fn event_format_is_deterministic_and_keeps_native_role_diagnostic() {
        let snapshot = ElementSnapshot {
            process_id: 4242,
            framework: "WinForm".into(),
            class_name: "WindowsForms10.EDIT.app.0.1".into(),
            role: "editable_text".into(),
            native_role: "edit".into(),
            control_type_id: 50004,
            name: "Runtime editable text".into(),
            automation_id: "RuntimeTextBox".into(),
        };

        assert_eq!(
            format_event_line(AccessibilityEventKind::Focus, 7, &snapshot),
            "FOCUS #7 | PID=4242 | Framework=WinForm | Class=WindowsForms10.EDIT.app.0.1 | Role=editable_text | Name=Runtime editable text | AutomationId=RuntimeTextBox | ControlTypeId=50004 | NativeRole=edit"
        );
    }
}
