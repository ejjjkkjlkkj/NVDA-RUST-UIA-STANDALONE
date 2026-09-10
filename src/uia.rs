use crate::semantic::Role;

// Microsoft UI Automation control type IDs are stable integer identifiers. Keeping the
// mapping in the portable core avoids using LocalizedControlType as semantic data.
pub fn role_from_control_type_id(control_type_id: i32) -> Role {
    match control_type_id {
        50000 => Role::Button,       // UIA_ButtonControlTypeId
        50002 => Role::CheckBox,     // UIA_CheckBoxControlTypeId
        50003 => Role::ComboBox,     // UIA_ComboBoxControlTypeId
        50004 => Role::EditableText, // UIA_EditControlTypeId
        50005 => Role::Link,         // UIA_HyperlinkControlTypeId
        50006 => Role::Image,        // UIA_ImageControlTypeId
        50007 => Role::ListItem,     // UIA_ListItemControlTypeId
        50008 => Role::List,         // UIA_ListControlTypeId
        50009 | 50010 => Role::Menu, // UIA_Menu / UIA_MenuBar
        50011 => Role::MenuItem,     // UIA_MenuItemControlTypeId
        50012 => Role::ProgressBar,  // UIA_ProgressBarControlTypeId
        50013 => Role::RadioButton,  // UIA_RadioButtonControlTypeId
        50015 => Role::Slider,       // UIA_SliderControlTypeId
        50018 => Role::TabPanel,     // UIA_TabControlTypeId
        50019 => Role::Tab,          // UIA_TabItemControlTypeId
        50020 => Role::StaticText,   // UIA_TextControlTypeId
        50021 | 50040 => Role::Toolbar, // UIA_ToolBar / UIA_AppBar
        50023 => Role::Tree,         // UIA_TreeControlTypeId
        50024 => Role::TreeItem,     // UIA_TreeItemControlTypeId
        50028 | 50036 => Role::Table, // UIA_DataGrid / UIA_Table
        50029 => Role::Row,          // UIA_DataItemControlTypeId
        50030 => Role::Document,     // UIA_DocumentControlTypeId
        50031 => Role::Button,       // UIA_SplitButtonControlTypeId
        50032 => Role::Window,       // UIA_WindowControlTypeId
        50035 => Role::Cell,         // UIA_HeaderItemControlTypeId
        _ => Role::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_interactive_control_types_are_locale_independent() {
        assert_eq!(role_from_control_type_id(50000), Role::Button);
        assert_eq!(role_from_control_type_id(50004), Role::EditableText);
        assert_eq!(role_from_control_type_id(50013), Role::RadioButton);
        assert_eq!(role_from_control_type_id(50024), Role::TreeItem);
        assert_eq!(role_from_control_type_id(50030), Role::Document);
        assert_eq!(role_from_control_type_id(50036), Role::Table);
    }

    #[test]
    fn tab_container_and_tab_item_remain_distinct() {
        assert_eq!(role_from_control_type_id(50018), Role::TabPanel);
        assert_eq!(role_from_control_type_id(50019), Role::Tab);
    }

    #[test]
    fn unknown_or_custom_control_types_do_not_guess_semantics() {
        assert_eq!(role_from_control_type_id(50025), Role::Unknown);
        assert_eq!(role_from_control_type_id(0), Role::Unknown);
        assert_eq!(role_from_control_type_id(i32::MAX), Role::Unknown);
    }
}
