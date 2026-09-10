use std::error::Error;

use axuielement::{AXUIElement, prelude::*};
use nvda_rust_uia_standalone::{
    presentation::focus_utterance,
    semantic::{AccessibleNode, Role, State},
};

mod semantic;

fn supports_attribute(attributes: &[String], name: &str) -> bool {
    attributes.iter().any(|attribute| attribute == name)
}

fn string_attribute(element: &AXUIElement, attributes: &[String], name: &str) -> String {
    if !supports_attribute(attributes, name) {
        return String::new();
    }

    element
        .string_attribute(name)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn bool_attribute(element: &AXUIElement, attributes: &[String], name: &str) -> Option<bool> {
    if !supports_attribute(attributes, name) {
        return None;
    }

    element.bool_attribute(name).ok().flatten()
}

fn i64_attribute(element: &AXUIElement, attributes: &[String], name: &str) -> Option<i64> {
    if !supports_attribute(attributes, name) {
        return None;
    }

    element.i64_attribute(name).ok().flatten()
}

fn value_is_settable(element: &AXUIElement, attributes: &[String]) -> Option<bool> {
    if !supports_attribute(attributes, "AXValue") {
        return None;
    }

    element.is_attribute_settable("AXValue").ok()
}

fn main() -> Result<(), Box<dyn Error>> {
    let api_enabled = api_enabled();
    let trusted = is_process_trusted();

    println!("AX_API_ENABLED = {api_enabled}");
    println!("AX_PROCESS_TRUSTED = {trusted}");

    let Some(system) = system_wide() else {
        return Err("no system-wide AXUIElement is available".into());
    };

    println!("AX_SYSTEM_WIDE = PASS");

    if !trusted {
        println!("AX_RUNTIME_PERMISSION = NOT_GRANTED");
        println!("AX_NATIVE_PROBE = PASS_PERMISSION_DIAGNOSTIC_ONLY");
        return Ok(());
    }

    let application = match system.focused_application() {
        Ok(application) => application,
        Err(error) => {
            println!("AX_INTERACTIVE_SESSION = UNAVAILABLE");
            println!("AX_FOCUSED_APPLICATION_ERROR = {error:?}");
            println!("AX_NATIVE_PROBE = PASS_API_ONLY");
            return Ok(());
        }
    };

    if let Some(application) = application {
        println!("AX_FOCUSED_APPLICATION_PID = {}", application.pid()?);
        println!(
            "AX_FOCUSED_APPLICATION_ATTRIBUTES = {}",
            application.attribute_names()?.len()
        );
    } else {
        println!("AX_FOCUSED_APPLICATION = NONE");
    }

    let element = match system.focused_ui_element() {
        Ok(element) => element,
        Err(error) => {
            println!("AX_INTERACTIVE_FOCUSED_ELEMENT = UNAVAILABLE");
            println!("AX_FOCUSED_ELEMENT_ERROR = {error:?}");
            println!("AX_NATIVE_PROBE = PASS_API_ONLY");
            return Ok(());
        }
    };

    if let Some(element) = element {
        let attributes = element.attribute_names()?;
        let native_role = string_attribute(&element, &attributes, "AXRole");
        let native_subrole = string_attribute(&element, &attributes, "AXSubrole");
        let title = string_attribute(&element, &attributes, "AXTitle");
        let description = string_attribute(&element, &attributes, "AXDescription");
        let role = semantic::role_from_ax(&native_role, &native_subrole);
        let is_password = native_subrole == semantic::AX_SECURE_TEXT_FIELD_SUBROLE;
        let value = if is_password {
            String::new()
        } else {
            string_attribute(&element, &attributes, "AXValue")
        };
        let states = semantic::states_from_ax(
            role,
            &native_subrole,
            semantic::AxStateSnapshot {
                focusable: supports_attribute(&attributes, "AXFocused"),
                focused: bool_attribute(&element, &attributes, "AXFocused").unwrap_or(false),
                enabled: bool_attribute(&element, &attributes, "AXEnabled"),
                selected: bool_attribute(&element, &attributes, "AXSelected"),
                expanded: bool_attribute(&element, &attributes, "AXExpanded"),
                check_value: if matches!(role, Role::CheckBox | Role::RadioButton) {
                    i64_attribute(&element, &attributes, "AXValue")
                } else {
                    None
                },
                value_settable: if role == Role::EditableText {
                    value_is_settable(&element, &attributes)
                } else {
                    None
                },
            },
        );
        let actions = element.action_names().unwrap_or_default();
        let parameterized_attributes = element.parameterized_attribute_names().unwrap_or_default();
        let children_count = if supports_attribute(&attributes, "AXChildren") {
            element.children().map_or(0, |children| children.len())
        } else {
            0
        };
        let process_id = i64::from(element.pid()?);

        let node = AccessibleNode {
            process_id,
            platform_id: format!("ax:{process_id}:{native_role}:{native_subrole}"),
            role,
            native_role: native_role.clone(),
            name: title.clone(),
            description: description.clone(),
            value,
            states,
            bounds: None,
        };

        println!("AX_FOCUSED_ROLE = {native_role}");
        println!("AX_FOCUSED_SUBROLE = {native_subrole}");
        println!("AX_SEMANTIC_ROLE = {role}");
        println!("AX_FOCUSED_TITLE = {title}");
        println!("AX_FOCUSED_DESCRIPTION = {description}");
        println!("AX_FOCUSED_ATTRIBUTES = {}", attributes.len());
        println!("AX_FOCUSED_ACTIONS = {}", actions.len());
        println!(
            "AX_FOCUSED_PARAMETERIZED_ATTRIBUTES = {}",
            parameterized_attributes.len()
        );
        println!("AX_FOCUSED_CHILDREN = {children_count}");
        println!("AX_SEMANTIC_STATES = {:?}", node.states);
        if node.has_state(State::Password) {
            println!("AX_FOCUSED_VALUE = <password>");
        } else {
            println!("AX_FOCUSED_VALUE = {}", node.value);
        }

        if let Some(utterance) = focus_utterance(&node) {
            println!("AX_PRESENTATION = {}", utterance.text);
        }
    } else {
        println!("AX_FOCUSED_ELEMENT = NONE");
    }

    println!("AX_INTERACTIVE_SESSION = PASS");
    println!("AX_NATIVE_PROBE = PASS");
    Ok(())
}
