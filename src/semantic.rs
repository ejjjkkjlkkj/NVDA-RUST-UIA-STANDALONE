use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Unknown,
    Application,
    Window,
    Dialog,
    Document,
    Heading,
    Paragraph,
    StaticText,
    EditableText,
    Button,
    CheckBox,
    RadioButton,
    ComboBox,
    List,
    ListItem,
    Tree,
    TreeItem,
    Table,
    Row,
    Cell,
    Link,
    Image,
    Slider,
    ProgressBar,
    Menu,
    MenuItem,
    Tab,
    TabPanel,
    Toolbar,
    Terminal,
}

impl Role {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Application => "application",
            Self::Window => "window",
            Self::Dialog => "dialog",
            Self::Document => "document",
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::StaticText => "static_text",
            Self::EditableText => "editable_text",
            Self::Button => "button",
            Self::CheckBox => "check_box",
            Self::RadioButton => "radio_button",
            Self::ComboBox => "combo_box",
            Self::List => "list",
            Self::ListItem => "list_item",
            Self::Tree => "tree",
            Self::TreeItem => "tree_item",
            Self::Table => "table",
            Self::Row => "row",
            Self::Cell => "cell",
            Self::Link => "link",
            Self::Image => "image",
            Self::Slider => "slider",
            Self::ProgressBar => "progress_bar",
            Self::Menu => "menu",
            Self::MenuItem => "menu_item",
            Self::Tab => "tab",
            Self::TabPanel => "tab_panel",
            Self::Toolbar => "toolbar",
            Self::Terminal => "terminal",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.canonical_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Focused,
    Focusable,
    Enabled,
    Disabled,
    Selected,
    Checked,
    Mixed,
    Expanded,
    Collapsed,
    ReadOnly,
    Editable,
    Required,
    Invalid,
    Busy,
    Multiline,
    Password,
    Offscreen,
}

impl State {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Focusable => "focusable",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Selected => "selected",
            Self::Checked => "checked",
            Self::Mixed => "mixed",
            Self::Expanded => "expanded",
            Self::Collapsed => "collapsed",
            Self::ReadOnly => "read_only",
            Self::Editable => "editable",
            Self::Required => "required",
            Self::Invalid => "invalid",
            Self::Busy => "busy",
            Self::Multiline => "multiline",
            Self::Password => "password",
            Self::Offscreen => "offscreen",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibleNode {
    pub process_id: i64,
    pub platform_id: String,
    pub role: Role,
    pub native_role: String,
    pub name: String,
    pub description: String,
    pub value: String,
    pub states: Vec<State>,
    pub bounds: Option<Rect>,
}

impl Default for AccessibleNode {
    fn default() -> Self {
        Self {
            process_id: 0,
            platform_id: String::new(),
            role: Role::Unknown,
            native_role: String::new(),
            name: String::new(),
            description: String::new(),
            value: String::new(),
            states: Vec::new(),
            bounds: None,
        }
    }
}

impl AccessibleNode {
    pub fn has_state(&self, state: State) -> bool {
        self.states.contains(&state)
    }

    pub fn primary_text(&self) -> &str {
        // Password providers are not trusted to keep secrets out of name or description.
        // Never surface any provider-controlled text from a password node.
        if self.has_state(State::Password) {
            return "";
        }

        if !self.name.is_empty() {
            &self.name
        } else if !self.value.is_empty() {
            &self.value
        } else if !self.description.is_empty() {
            &self.description
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_names_are_platform_neutral() {
        assert_eq!(Role::Button.canonical_name(), "button");
        assert_eq!(Role::EditableText.canonical_name(), "editable_text");
        assert_eq!(Role::Terminal.canonical_name(), "terminal");
    }

    #[test]
    fn name_has_priority_for_presentation() {
        let node = AccessibleNode {
            name: "Save".into(),
            value: "ignored".into(),
            ..AccessibleNode::default()
        };
        assert_eq!(node.primary_text(), "Save");
    }

    #[test]
    fn password_provider_text_is_never_primary_text() {
        let node = AccessibleNode {
            name: "secret-name".into(),
            value: "secret-value".into(),
            description: "secret-description".into(),
            states: vec![State::Password],
            ..AccessibleNode::default()
        };
        assert_eq!(node.primary_text(), "");
    }
}
