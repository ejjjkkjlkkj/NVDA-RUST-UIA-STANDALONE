use crate::semantic::{AccessibleNode, Role, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechPriority {
    Background,
    Normal,
    Focus,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    pub text: String,
    pub priority: SpeechPriority,
    pub interrupt: bool,
}

fn push_token(tokens: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    let value = value.trim();
    if !value.is_empty() && !tokens.iter().any(|token| token == value) {
        tokens.push(value.to_string());
    }
}

pub fn focus_utterance(node: &AccessibleNode) -> Option<Utterance> {
    let mut tokens = Vec::new();

    push_token(&mut tokens, node.name.clone());

    if node.role != Role::Unknown {
        push_token(&mut tokens, node.role.speech_name());
    } else {
        push_token(&mut tokens, node.native_role.clone());
    }

    if node.has_state(State::Password) {
        push_token(&mut tokens, "protected");
    } else if node.role.reports_value_on_focus() {
        push_token(&mut tokens, node.safe_value());
    }

    if node.has_state(State::Checked) {
        push_token(&mut tokens, "checked");
    } else if node.has_state(State::Mixed) {
        push_token(&mut tokens, "partially checked");
    }

    if node.has_state(State::Selected) {
        push_token(&mut tokens, "selected");
    }
    if node.has_state(State::Expanded) {
        push_token(&mut tokens, "expanded");
    } else if node.has_state(State::Collapsed) {
        push_token(&mut tokens, "collapsed");
    }
    if node.has_state(State::Required) {
        push_token(&mut tokens, "required");
    }
    if node.has_state(State::Invalid) {
        push_token(&mut tokens, "invalid");
    }
    if node.has_state(State::Disabled) {
        push_token(&mut tokens, "disabled");
    }

    if tokens.is_empty() {
        push_token(&mut tokens, node.description.clone());
    }

    if tokens.is_empty() {
        return None;
    }

    Some(Utterance {
        text: tokens.join(", "),
        priority: SpeechPriority::Focus,
        interrupt: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_presentation_combines_semantics() {
        let node = AccessibleNode {
            role: Role::CheckBox,
            name: "Remember me".into(),
            states: vec![State::Focusable, State::Checked],
            ..AccessibleNode::default()
        };

        let utterance = focus_utterance(&node).expect("utterance");
        assert_eq!(utterance.text, "Remember me, check box, checked");
        assert_eq!(utterance.priority, SpeechPriority::Focus);
        assert!(utterance.interrupt);
    }

    #[test]
    fn edit_focus_reports_label_role_and_value() {
        let node = AccessibleNode {
            role: Role::EditableText,
            name: "Search".into(),
            value: "rust accessibility".into(),
            states: vec![State::Focusable, State::Editable],
            ..AccessibleNode::default()
        };

        let utterance = focus_utterance(&node).expect("utterance");
        assert_eq!(utterance.text, "Search, edit, rust accessibility");
    }

    #[test]
    fn duplicate_value_is_not_repeated() {
        let node = AccessibleNode {
            role: Role::ComboBox,
            name: "Detailed".into(),
            value: "Detailed".into(),
            ..AccessibleNode::default()
        };

        let utterance = focus_utterance(&node).expect("utterance");
        assert_eq!(utterance.text, "Detailed, combo box");
    }

    #[test]
    fn password_value_is_not_spoken() {
        let node = AccessibleNode {
            role: Role::EditableText,
            name: "Password".into(),
            value: "super-secret".into(),
            states: vec![State::Password],
            ..AccessibleNode::default()
        };

        let utterance = focus_utterance(&node).expect("role still produces speech");
        assert_eq!(utterance.text, "Password, edit, protected");
        assert!(!utterance.text.contains("super-secret"));
    }

    #[test]
    fn unknown_empty_node_produces_no_speech() {
        assert_eq!(focus_utterance(&AccessibleNode::default()), None);
    }
}
