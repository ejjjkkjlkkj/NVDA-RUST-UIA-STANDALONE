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
    if !value.trim().is_empty() {
        tokens.push(value);
    }
}

pub fn focus_utterance(node: &AccessibleNode) -> Option<Utterance> {
    let mut tokens = Vec::new();

    if node.has_state(State::Password) {
        // Use a fixed phrase before consuming any provider-controlled text. A broken or
        // hostile accessibility provider must not be able to leak a password through
        // name, value, or description.
        push_token(&mut tokens, "password field");
    } else {
        push_token(&mut tokens, node.primary_text());
    }

    if node.role != Role::Unknown {
        push_token(&mut tokens, node.role.to_string());
    } else {
        push_token(&mut tokens, node.native_role.clone());
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
        assert_eq!(utterance.text, "Remember me, check_box, checked");
        assert_eq!(utterance.priority, SpeechPriority::Focus);
        assert!(utterance.interrupt);
    }

    #[test]
    fn password_provider_text_is_never_spoken() {
        let node = AccessibleNode {
            role: Role::EditableText,
            name: "super-secret-name".into(),
            value: "super-secret-value".into(),
            description: "super-secret-description".into(),
            states: vec![State::Password],
            ..AccessibleNode::default()
        };

        let utterance = focus_utterance(&node).expect("password role produces safe speech");
        assert_eq!(utterance.text, "password field, editable_text");
        assert!(!utterance.text.contains("super-secret"));
    }

    #[test]
    fn unknown_empty_node_produces_no_speech() {
        assert_eq!(focus_utterance(&AccessibleNode::default()), None);
    }
}
