use std::{env, error::Error, time::Duration};

use atspi::{AccessibilityConnection, Event, FocusEvents, ObjectEvents, WindowEvents};
use futures_util::StreamExt;
use tokio::time::timeout;

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
            }
            Ok(Some(Err(error))) => {
                eprintln!("AT_SPI_EVENT_ERROR = {error}");
            }
            Ok(None) | Err(_) => break,
        }
    }

    println!(
        "AT_SPI_COUNTS total={total_events} focus={focus_events} object={object_events} window={window_events}"
    );

    if total_events == 0 {
        return Err("no AT-SPI events were observed during the runtime window".into());
    }

    println!("AT_SPI_RUNTIME = PASS");
    Ok(())
}
