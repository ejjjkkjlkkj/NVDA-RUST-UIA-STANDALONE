use std::{env, error::Error, time::Duration};

use atspi::{
    AccessibilityConnection, Event, EventProperties, FocusEvents, ObjectEvents, WindowEvents,
    connection::P2P,
};
use futures_util::StreamExt;
use nvda_rust_uia_standalone::{presentation::focus_utterance, semantic::AccessibleNode};
use tokio::time::timeout;

mod semantic;

const DEFAULT_MONITOR_SECONDS: u64 = 8;

fn monitor_seconds() -> u64 {
    env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MONITOR_SECONDS)
}

fn event_family(event: &Event) -> &'static str {
    match event {
        Event::Focus(_) => "focus",
        Event::Object(_) => "object",
        Event::Window(_) => "window",
        Event::Document(_) => "document",
        Event::Keyboard(_) => "keyboard",
        Event::Mouse(_) => "mouse",
        Event::Terminal(_) => "terminal",
        Event::Available(_) => "available",
        Event::Cache(_) => "cache",
        Event::Listener(_) => "listener",
        _ => "unknown",
    }
}

async fn semantic_node_for_event(
    connection: &AccessibilityConnection,
    event: &Event,
) -> Result<Option<AccessibleNode>, Box<dyn Error>> {
    let object_ref = event.object_ref();
    if object_ref.is_null() {
        return Ok(None);
    }

    let object_ref = object_ref.into();
    let accessible = connection.object_as_accessible(&object_ref).await?;
    let native_role = accessible.get_role().await?;
    let native_states = accessible.get_state().await?;
    let name = accessible.name().await.unwrap_or_default();
    let description = accessible.description().await.unwrap_or_default();
    let accessible_id = accessible.accessible_id().await.unwrap_or_default();
    let role = semantic::role_from_atspi(native_role, native_states);
    let states = semantic::states_from_atspi(native_role, native_states);
    let bus_name = object_ref.name_as_str().unwrap_or("unknown");
    let path = object_ref.path_as_str();

    Ok(Some(AccessibleNode {
        process_id: 0,
        platform_id: if accessible_id.is_empty() {
            format!("atspi:{bus_name}:{path}")
        } else {
            format!("atspi:{bus_name}:{path}:{accessible_id}")
        },
        role,
        native_role: native_role.to_string(),
        name,
        description,
        value: String::new(),
        states,
        bounds: None,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let seconds = monitor_seconds();
    let connection = AccessibilityConnection::new().await?;

    let root = connection.root_accessible_on_registry().await?;
    let children = root.get_children().await?;

    println!("AT_SPI_CONNECTION = PASS");
    println!("AT_SPI_ROOT_CHILDREN = {}", children.len());

    connection.register_event::<FocusEvents>().await?;
    connection.register_event::<ObjectEvents>().await?;
    connection.register_event::<WindowEvents>().await?;

    println!("AT_SPI_EVENTS_REGISTERED = focus,object,window");
    println!("MONITOR_SECONDS = {seconds}");

    if seconds == 0 {
        println!("AT_SPI_RUNTIME = PASS");
        return Ok(());
    }

    let mut stream = Box::pin(connection.event_stream());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut total_events = 0_u64;
    let mut focus_events = 0_u64;
    let mut object_events = 0_u64;
    let mut window_events = 0_u64;
    let mut semantic_events = 0_u64;

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }

        let remaining = deadline.saturating_duration_since(now);
        match timeout(remaining, stream.next()).await {
            Ok(Some(Ok(event))) => {
                total_events += 1;
                match &event {
                    Event::Focus(_) => focus_events += 1,
                    Event::Object(_) => object_events += 1,
                    Event::Window(_) => window_events += 1,
                    _ => {}
                }
                println!(
                    "AT_SPI_EVENT #{} family={} event={event:?}",
                    total_events,
                    event_family(&event)
                );

                match semantic_node_for_event(&connection, &event).await {
                    Ok(Some(node)) => {
                        semantic_events += 1;
                        println!(
                            "AT_SPI_SEMANTIC #{} role={} native_role={} name={} states={:?} platform_id={}",
                            semantic_events,
                            node.role,
                            node.native_role,
                            node.name,
                            node.states,
                            node.platform_id
                        );

                        if matches!(event, Event::Focus(_)) {
                            if let Some(utterance) = focus_utterance(&node) {
                                println!("AT_SPI_PRESENTATION = {}", utterance.text);
                            }
                        }
                    }
                    Ok(None) => {
                        println!("AT_SPI_SEMANTIC_SKIPPED = NULL_OBJECT");
                    }
                    Err(error) => {
                        eprintln!("AT_SPI_SEMANTIC_ERROR = {error}");
                    }
                }
            }
            Ok(Some(Err(error))) => {
                eprintln!("AT_SPI_EVENT_ERROR = {error}");
            }
            Ok(None) | Err(_) => break,
        }
    }

    println!(
        "AT_SPI_COUNTS total={total_events} focus={focus_events} object={object_events} window={window_events} semantic={semantic_events}"
    );

    if total_events == 0 {
        return Err("no AT-SPI events were observed during the runtime window".into());
    }

    println!("AT_SPI_RUNTIME = PASS");
    Ok(())
}
