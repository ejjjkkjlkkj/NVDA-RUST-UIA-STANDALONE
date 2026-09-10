use atspi::{Role as AtspiRole, State as AtspiState, StateSet};
use nvda_rust_uia_standalone::semantic::{Role, State};

pub fn role_from_atspi(role: AtspiRole, states: StateSet) -> Role {
    match role {
        AtspiRole::Application => Role::Application,
        AtspiRole::Window | AtspiRole::Frame => Role::Window,
        AtspiRole::Dialog | AtspiRole::Alert => Role::Dialog,
        AtspiRole::DocumentFrame
        | AtspiRole::DocumentText
        | AtspiRole::DocumentWeb
        | AtspiRole::DocumentEmail
        | AtspiRole::DocumentPresentation
        | AtspiRole::DocumentSpreadsheet => Role::Document,
        AtspiRole::Heading => Role::Heading,
        AtspiRole::Paragraph => Role::Paragraph,
        AtspiRole::Static | AtspiRole::Label => Role::StaticText,
        AtspiRole::Entry | AtspiRole::Editbar | AtspiRole::PasswordText => Role::EditableText,
        AtspiRole::Text if states.contains(AtspiState::Editable) => Role::EditableText,
        AtspiRole::Text => Role::StaticText,
        AtspiRole::Button | AtspiRole::ToggleButton | AtspiRole::PushButtonMenu => Role::Button,
        AtspiRole::CheckBox => Role::CheckBox,
        AtspiRole::RadioButton => Role::RadioButton,
        AtspiRole::ComboBox | AtspiRole::SpinButton => Role::ComboBox,
        AtspiRole::List | AtspiRole::ListBox => Role::List,
        AtspiRole::ListItem => Role::ListItem,
        AtspiRole::Tree => Role::Tree,
        AtspiRole::TreeItem => Role::TreeItem,
        AtspiRole::Table | AtspiRole::TreeTable => Role::Table,
        AtspiRole::TableRow => Role::Row,
        AtspiRole::TableCell | AtspiRole::TableColumnHeader | AtspiRole::TableRowHeader => Role::Cell,
        AtspiRole::Link => Role::Link,
        AtspiRole::Image | AtspiRole::ImageMap => Role::Image,
        AtspiRole::Slider => Role::Slider,
        AtspiRole::ProgressBar | AtspiRole::LevelBar => Role::ProgressBar,
        AtspiRole::MenuBar | AtspiRole::PopupMenu => Role::Menu,
        AtspiRole::MenuItem
        | AtspiRole::CheckMenuItem
        | AtspiRole::RadioMenuItem
        | AtspiRole::TearoffMenuItem => Role::MenuItem,
        AtspiRole::PageTab => Role::Tab,
        AtspiRole::PageTabList => Role::TabPanel,
        AtspiRole::ToolBar => Role::Toolbar,
        AtspiRole::Terminal => Role::Terminal,
        _ => Role::Unknown,
    }
}

pub fn states_from_atspi(role: AtspiRole, states: StateSet) -> Vec<State> {
    let mut result = Vec::new();

    if states.contains(AtspiState::Focused) {
        result.push(State::Focused);
    }
    if states.contains(AtspiState::Focusable) {
        result.push(State::Focusable);
    }
    if states.contains(AtspiState::Enabled) {
        result.push(State::Enabled);
    } else {
        result.push(State::Disabled);
    }
    if states.contains(AtspiState::Selected) {
        result.push(State::Selected);
    }
    if states.contains(AtspiState::Checked) {
        result.push(State::Checked);
    }
    if states.contains(AtspiState::Indeterminate) {
        result.push(State::Mixed);
    }
    if states.contains(AtspiState::Expanded) {
        result.push(State::Expanded);
    } else if states.contains(AtspiState::Collapsed) {
        result.push(State::Collapsed);
    }
    if states.contains(AtspiState::ReadOnly) {
        result.push(State::ReadOnly);
    }
    if states.contains(AtspiState::Editable) {
        result.push(State::Editable);
    }
    if states.contains(AtspiState::Required) {
        result.push(State::Required);
    }
    if states.contains(AtspiState::Invalid) || states.contains(AtspiState::InvalidEntry) {
        result.push(State::Invalid);
    }
    if states.contains(AtspiState::Busy) {
        result.push(State::Busy);
    }
    if states.contains(AtspiState::MultiLine) {
        result.push(State::Multiline);
    }
    if role == AtspiRole::PasswordText {
        result.push(State::Password);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_core_roles() {
        assert_eq!(role_from_atspi(AtspiRole::Button, StateSet::empty()), Role::Button);
        assert_eq!(role_from_atspi(AtspiRole::Heading, StateSet::empty()), Role::Heading);
        assert_eq!(role_from_atspi(AtspiRole::Terminal, StateSet::empty()), Role::Terminal);
    }

    #[test]
    fn text_role_uses_editable_state() {
        let editable = StateSet::new(AtspiState::Editable);
        assert_eq!(role_from_atspi(AtspiRole::Text, editable), Role::EditableText);
        assert_eq!(role_from_atspi(AtspiRole::Text, StateSet::empty()), Role::StaticText);
    }

    #[test]
    fn password_state_is_explicit() {
        let states = states_from_atspi(AtspiRole::PasswordText, StateSet::new(AtspiState::Focusable));
        assert!(states.contains(&State::Password));
        assert!(states.contains(&State::Focusable));
    }
}
