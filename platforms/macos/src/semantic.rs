use nvda_rust_uia_standalone::semantic::{Role, State};

pub const AX_SECURE_TEXT_FIELD_SUBROLE: &str = "AXSecureTextField";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AxStateSnapshot {
    pub focusable: bool,
    pub focused: bool,
    pub enabled: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub check_value: Option<i64>,
    pub value_settable: Option<bool>,
}

pub fn role_from_ax(native_role: &str, native_subrole: &str) -> Role {
    if native_subrole == AX_SECURE_TEXT_FIELD_SUBROLE {
        return Role::EditableText;
    }

    match native_role {
        "AXApplication" => Role::Application,
        "AXWindow" => Role::Window,
        "AXDialog" | "AXSheet" => Role::Dialog,
        "AXWebArea" | "AXDocument" => Role::Document,
        "AXHeading" => Role::Heading,
        "AXStaticText" => Role::StaticText,
        "AXTextField" | "AXTextArea" => Role::EditableText,
        "AXButton" => Role::Button,
        "AXCheckBox" => Role::CheckBox,
        "AXRadioButton" => Role::RadioButton,
        "AXComboBox" | "AXPopUpButton" => Role::ComboBox,
        "AXList" => Role::List,
        "AXRow" => Role::Row,
        "AXOutline" => Role::Tree,
        "AXTable" | "AXGrid" => Role::Table,
        "AXCell" => Role::Cell,
        "AXLink" => Role::Link,
        "AXImage" => Role::Image,
        "AXSlider" => Role::Slider,
        "AXProgressIndicator" => Role::ProgressBar,
        "AXMenu" => Role::Menu,
        "AXMenuItem" | "AXMenuButton" => Role::MenuItem,
        "AXTabGroup" => Role::TabPanel,
        "AXToolbar" => Role::Toolbar,
        _ => Role::Unknown,
    }
}

pub fn states_from_ax(
    role: Role,
    native_subrole: &str,
    snapshot: AxStateSnapshot,
) -> Vec<State> {
    let mut states = Vec::new();

    if snapshot.focusable {
        states.push(State::Focusable);
    }
    if snapshot.focused {
        states.push(State::Focused);
    }
    match snapshot.enabled {
        Some(true) => states.push(State::Enabled),
        Some(false) => states.push(State::Disabled),
        None => {}
    }
    if snapshot.selected == Some(true) {
        states.push(State::Selected);
    }
    match snapshot.expanded {
        Some(true) => states.push(State::Expanded),
        Some(false) => states.push(State::Collapsed),
        None => {}
    }
    match snapshot.check_value {
        Some(1) => states.push(State::Checked),
        Some(2) => states.push(State::Mixed),
        _ => {}
    }
    if role == Role::EditableText {
        match snapshot.value_settable {
            Some(true) => states.push(State::Editable),
            Some(false) => states.push(State::ReadOnly),
            None => {}
        }
    }
    if native_subrole == AX_SECURE_TEXT_FIELD_SUBROLE {
        states.push(State::Password);
    }

    states
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_ax_roles() {
        assert_eq!(role_from_ax("AXButton", ""), Role::Button);
        assert_eq!(role_from_ax("AXTextField", ""), Role::EditableText);
        assert_eq!(role_from_ax("AXWebArea", ""), Role::Document);
        assert_eq!(role_from_ax("AXTable", ""), Role::Table);
    }

    #[test]
    fn maps_secure_field_and_password_state() {
        let role = role_from_ax("AXTextField", AX_SECURE_TEXT_FIELD_SUBROLE);
        let states = states_from_ax(
            role,
            AX_SECURE_TEXT_FIELD_SUBROLE,
            AxStateSnapshot {
                focusable: true,
                focused: true,
                enabled: Some(true),
                value_settable: Some(true),
                ..AxStateSnapshot::default()
            },
        );

        assert_eq!(role, Role::EditableText);
        assert!(states.contains(&State::Password));
        assert!(states.contains(&State::Editable));
        assert!(states.contains(&State::Focused));
    }

    #[test]
    fn maps_control_states() {
        let states = states_from_ax(
            Role::CheckBox,
            "",
            AxStateSnapshot {
                focusable: true,
                enabled: Some(false),
                selected: Some(true),
                expanded: Some(false),
                check_value: Some(2),
                ..AxStateSnapshot::default()
            },
        );

        assert!(states.contains(&State::Focusable));
        assert!(states.contains(&State::Disabled));
        assert!(states.contains(&State::Selected));
        assert!(states.contains(&State::Collapsed));
        assert!(states.contains(&State::Mixed));
    }

    #[test]
    fn preserves_unknown_native_roles_as_unknown() {
        assert_eq!(role_from_ax("AXFutureRole", ""), Role::Unknown);
    }
}
