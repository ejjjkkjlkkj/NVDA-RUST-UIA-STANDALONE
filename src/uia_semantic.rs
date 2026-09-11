use crate::semantic::{AccessibleNode, Role, State};

pub const UIA_BUTTON_CONTROL_TYPE_ID: i32 = 50000;
pub const UIA_CHECK_BOX_CONTROL_TYPE_ID: i32 = 50002;
pub const UIA_COMBO_BOX_CONTROL_TYPE_ID: i32 = 50003;
pub const UIA_EDIT_CONTROL_TYPE_ID: i32 = 50004;
pub const UIA_HYPERLINK_CONTROL_TYPE_ID: i32 = 50005;
pub const UIA_IMAGE_CONTROL_TYPE_ID: i32 = 50006;
pub const UIA_LIST_ITEM_CONTROL_TYPE_ID: i32 = 50007;
pub const UIA_LIST_CONTROL_TYPE_ID: i32 = 50008;
pub const UIA_MENU_CONTROL_TYPE_ID: i32 = 50009;
pub const UIA_MENU_BAR_CONTROL_TYPE_ID: i32 = 50010;
pub const UIA_MENU_ITEM_CONTROL_TYPE_ID: i32 = 50011;
pub const UIA_PROGRESS_BAR_CONTROL_TYPE_ID: i32 = 50012;
pub const UIA_RADIO_BUTTON_CONTROL_TYPE_ID: i32 = 50013;
pub const UIA_SLIDER_CONTROL_TYPE_ID: i32 = 50015;
pub const UIA_STATUS_BAR_CONTROL_TYPE_ID: i32 = 50017;
pub const UIA_TAB_CONTROL_TYPE_ID: i32 = 50018;
pub const UIA_TAB_ITEM_CONTROL_TYPE_ID: i32 = 50019;
pub const UIA_TEXT_CONTROL_TYPE_ID: i32 = 50020;
pub const UIA_TOOL_BAR_CONTROL_TYPE_ID: i32 = 50021;
pub const UIA_TOOL_TIP_CONTROL_TYPE_ID: i32 = 50022;
pub const UIA_TREE_CONTROL_TYPE_ID: i32 = 50023;
pub const UIA_TREE_ITEM_CONTROL_TYPE_ID: i32 = 50024;
pub const UIA_DATA_GRID_CONTROL_TYPE_ID: i32 = 50028;
pub const UIA_DATA_ITEM_CONTROL_TYPE_ID: i32 = 50029;
pub const UIA_DOCUMENT_CONTROL_TYPE_ID: i32 = 50030;
pub const UIA_SPLIT_BUTTON_CONTROL_TYPE_ID: i32 = 50031;
pub const UIA_WINDOW_CONTROL_TYPE_ID: i32 = 50032;
pub const UIA_HEADER_CONTROL_TYPE_ID: i32 = 50034;
pub const UIA_HEADER_ITEM_CONTROL_TYPE_ID: i32 = 50035;
pub const UIA_TABLE_CONTROL_TYPE_ID: i32 = 50036;
pub const UIA_TITLE_BAR_CONTROL_TYPE_ID: i32 = 50037;
pub const UIA_APP_BAR_CONTROL_TYPE_ID: i32 = 50040;

#[derive(Debug, Clone, Copy)]
pub struct UiaSemanticInput<'a> {
    pub process_id: i32,
    pub platform_id: &'a str,
    pub control_type_id: i32,
    pub native_role: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub value: &'a str,
    pub is_password: bool,
    pub is_enabled: bool,
    pub is_keyboard_focusable: bool,
    pub has_keyboard_focus: bool,
    pub is_offscreen: bool,
    pub is_selected: Option<bool>,
    /// UIA ToggleState: 0=off, 1=on, 2=indeterminate.
    pub toggle_state: Option<i32>,
    /// UIA ExpandCollapseState: 0=collapsed, 1=expanded, 2=partially expanded, 3=leaf.
    pub expand_collapse_state: Option<i32>,
}

pub const fn role_from_control_type_id(control_type_id: i32) -> Role {
    match control_type_id {
        UIA_BUTTON_CONTROL_TYPE_ID | UIA_SPLIT_BUTTON_CONTROL_TYPE_ID => Role::Button,
        UIA_CHECK_BOX_CONTROL_TYPE_ID => Role::CheckBox,
        UIA_COMBO_BOX_CONTROL_TYPE_ID => Role::ComboBox,
        UIA_EDIT_CONTROL_TYPE_ID => Role::EditableText,
        UIA_HYPERLINK_CONTROL_TYPE_ID => Role::Link,
        UIA_IMAGE_CONTROL_TYPE_ID => Role::Image,
        UIA_LIST_ITEM_CONTROL_TYPE_ID => Role::ListItem,
        UIA_LIST_CONTROL_TYPE_ID => Role::List,
        UIA_MENU_CONTROL_TYPE_ID | UIA_MENU_BAR_CONTROL_TYPE_ID => Role::Menu,
        UIA_MENU_ITEM_CONTROL_TYPE_ID => Role::MenuItem,
        UIA_PROGRESS_BAR_CONTROL_TYPE_ID => Role::ProgressBar,
        UIA_RADIO_BUTTON_CONTROL_TYPE_ID => Role::RadioButton,
        UIA_SLIDER_CONTROL_TYPE_ID => Role::Slider,
        UIA_STATUS_BAR_CONTROL_TYPE_ID
        | UIA_TEXT_CONTROL_TYPE_ID
        | UIA_TOOL_TIP_CONTROL_TYPE_ID
        | UIA_TITLE_BAR_CONTROL_TYPE_ID => Role::StaticText,
        UIA_TAB_CONTROL_TYPE_ID => Role::TabPanel,
        UIA_TAB_ITEM_CONTROL_TYPE_ID => Role::Tab,
        UIA_TOOL_BAR_CONTROL_TYPE_ID | UIA_APP_BAR_CONTROL_TYPE_ID => Role::Toolbar,
        UIA_TREE_CONTROL_TYPE_ID => Role::Tree,
        UIA_TREE_ITEM_CONTROL_TYPE_ID => Role::TreeItem,
        UIA_DATA_GRID_CONTROL_TYPE_ID | UIA_TABLE_CONTROL_TYPE_ID => Role::Table,
        UIA_DATA_ITEM_CONTROL_TYPE_ID | UIA_HEADER_CONTROL_TYPE_ID => Role::Row,
        UIA_HEADER_ITEM_CONTROL_TYPE_ID => Role::Cell,
        UIA_DOCUMENT_CONTROL_TYPE_ID => Role::Document,
        UIA_WINDOW_CONTROL_TYPE_ID => Role::Window,
        _ => Role::Unknown,
    }
}

fn push_state(states: &mut Vec<State>, state: State) {
    if !states.contains(&state) {
        states.push(state);
    }
}

pub fn build_accessible_node(input: UiaSemanticInput<'_>) -> AccessibleNode {
    let role = role_from_control_type_id(input.control_type_id);
    let mut states = Vec::new();

    if input.has_keyboard_focus {
        push_state(&mut states, State::Focused);
    }
    if input.is_keyboard_focusable {
        push_state(&mut states, State::Focusable);
    }
    if input.is_enabled {
        push_state(&mut states, State::Enabled);
    } else {
        push_state(&mut states, State::Disabled);
    }
    if input.is_offscreen {
        push_state(&mut states, State::Offscreen);
    }
    if input.is_password {
        push_state(&mut states, State::Password);
    }
    if role == Role::EditableText {
        push_state(&mut states, State::Editable);
    }
    if input.is_selected == Some(true) {
        push_state(&mut states, State::Selected);
    }

    match input.toggle_state {
        Some(1) => push_state(&mut states, State::Checked),
        Some(2) => push_state(&mut states, State::Mixed),
        _ => {}
    }

    match input.expand_collapse_state {
        Some(0) => push_state(&mut states, State::Collapsed),
        Some(1) | Some(2) => push_state(&mut states, State::Expanded),
        _ => {}
    }

    AccessibleNode {
        process_id: i64::from(input.process_id),
        platform_id: input.platform_id.to_string(),
        role,
        native_role: input.native_role.to_string(),
        name: input.name.to_string(),
        description: input.description.to_string(),
        value: if input.is_password {
            String::new()
        } else {
            input.value.to_string()
        },
        states,
        bounds: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(control_type_id: i32) -> UiaSemanticInput<'static> {
        UiaSemanticInput {
            process_id: 42,
            platform_id: "opaque-id",
            control_type_id,
            native_role: "native",
            name: "Name",
            description: "Description",
            value: "Value",
            is_password: false,
            is_enabled: true,
            is_keyboard_focusable: true,
            has_keyboard_focus: true,
            is_offscreen: false,
            is_selected: None,
            toggle_state: None,
            expand_collapse_state: None,
        }
    }

    #[test]
    fn common_control_types_map_to_platform_neutral_roles() {
        assert_eq!(role_from_control_type_id(50000), Role::Button);
        assert_eq!(role_from_control_type_id(50002), Role::CheckBox);
        assert_eq!(role_from_control_type_id(50004), Role::EditableText);
        assert_eq!(role_from_control_type_id(50005), Role::Link);
        assert_eq!(role_from_control_type_id(50030), Role::Document);
        assert_eq!(role_from_control_type_id(50036), Role::Table);
        assert_eq!(role_from_control_type_id(99999), Role::Unknown);
    }

    #[test]
    fn password_value_is_removed_at_adapter_boundary() {
        let mut input = base(UIA_EDIT_CONTROL_TYPE_ID);
        input.is_password = true;
        input.value = "must-never-cross-boundary";
        let node = build_accessible_node(input);
        assert!(node.has_state(State::Password));
        assert!(node.has_state(State::Editable));
        assert_eq!(node.value, "");
    }

    #[test]
    fn toggle_and_expand_states_are_semantic() {
        let mut input = base(UIA_CHECK_BOX_CONTROL_TYPE_ID);
        input.toggle_state = Some(1);
        input.expand_collapse_state = Some(0);
        let node = build_accessible_node(input);
        assert!(node.has_state(State::Checked));
        assert!(node.has_state(State::Collapsed));
    }

    #[test]
    fn disabled_is_explicit_and_not_enabled() {
        let mut input = base(UIA_BUTTON_CONTROL_TYPE_ID);
        input.is_enabled = false;
        let node = build_accessible_node(input);
        assert!(node.has_state(State::Disabled));
        assert!(!node.has_state(State::Enabled));
    }
}
