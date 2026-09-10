use std::error::Error;

use axuielement::{
    ax_attribute::{AX_ROLE_ATTRIBUTE, AX_TITLE_ATTRIBUTE},
    prelude::*,
};

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

    if let Some(application) = system.focused_application()? {
        println!("AX_FOCUSED_APPLICATION_PID = {}", application.pid()?);
        println!(
            "AX_FOCUSED_APPLICATION_ATTRIBUTES = {}",
            application.attribute_names()?.len()
        );
    } else {
        println!("AX_FOCUSED_APPLICATION = NONE");
    }

    if let Some(element) = system.focused_ui_element()? {
        let role = element
            .string_attribute(AX_ROLE_ATTRIBUTE)?
            .unwrap_or_default();
        let title = element
            .string_attribute(AX_TITLE_ATTRIBUTE)?
            .unwrap_or_default();
        let actions = element.action_names()?;

        println!("AX_FOCUSED_ROLE = {role}");
        println!("AX_FOCUSED_TITLE = {title}");
        println!("AX_FOCUSED_ACTIONS = {}", actions.len());
    } else {
        println!("AX_FOCUSED_ELEMENT = NONE");
    }

    println!("AX_NATIVE_PROBE = PASS");
    Ok(())
}
