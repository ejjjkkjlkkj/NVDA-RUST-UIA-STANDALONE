use nvda_rust_uia_standalone::semantic::Role;

pub fn role_from_ax(native_role: &str) -> Role {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_ax_roles() {
        assert_eq!(role_from_ax("AXButton"), Role::Button);
        assert_eq!(role_from_ax("AXTextField"), Role::EditableText);
        assert_eq!(role_from_ax("AXWebArea"), Role::Document);
        assert_eq!(role_from_ax("AXTable"), Role::Table);
    }

    #[test]
    fn preserves_unknown_native_roles_as_unknown() {
        assert_eq!(role_from_ax("AXFutureRole"), Role::Unknown);
    }
}
