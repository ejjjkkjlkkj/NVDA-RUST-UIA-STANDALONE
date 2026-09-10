use std::error::Error;

use axuielement::{
    ax_attribute::{AX_ROLE_ATTRIBUTE, AX_TITLE_ATTRIBUTE},
    prelude::*,
};
use nvda_rust_uia_standalone::{presentation::focus_utterance, semantic::AccessibleNode};

mod semantic;

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
        let native_role = element
            .string_attribute(AX_ROLE_ATTRIBUTE)?
            .unwrap_or_default();
        let title = element
            .string_attribute(AX_TITLE_ATTRIBUTE)?
            .unwrap_or_default();
        let actions = element.action_names()?;
        let process_id = i64::from(element.pid()?);
        let role = semantic::role_from_ax(&native_role);

        let node = AccessibleNode {
            process_id,
            platform_id: format!("ax:{process_id}:{native_role}"),
            role,
            native_role: native_role.clone(),
            name: title.clone(),
            ..AccessibleNode::default()
        };

        println!("AX_FOCUSED_ROLE = {native_role}");
        println!("AX_SEMANTIC_ROLE = {role}");
        println!("AX_FOCUSED_TITLE = {title}");
        println!("AX_FOCUSED_ACTIONS = {}", actions.len());

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
