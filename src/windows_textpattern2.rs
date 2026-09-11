use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use windows::Win32::*;

const TEXT_PATTERN2_PATTERN: PATTERNID = PATTERNID(10024);
const MAX_SELECTION_STATES: usize = 256;
const MAX_SELECTION_SPEECH_CHARS: usize = 200;

static TEXT_PATTERN2_COUNT: AtomicU64 = AtomicU64::new(0);
static CARET_ACTIVE_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_TEXT_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_PATTERN2_PROTECTED_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_SPEECH_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_SELECTED_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_UNSELECTED_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_STATES: Mutex<Vec<SelectionState>> = Mutex::new(Vec::new());

struct SelectionState {
    identity: String,
    text: String,
}

fn element_identity(element: &IUIAutomationElement, process_id: i32) -> String {
    unsafe {
        let automation_id = element
            .CurrentAutomationId()
            .map(|value| value.display().to_string())
            .unwrap_or_default();
        let name = element
            .CurrentName()
            .map(|value| value.display().to_string())
            .unwrap_or_default();
        let class_name = element
            .CurrentClassName()
            .map(|value| value.display().to_string())
            .unwrap_or_default();
        let process_id = process_id.to_string();
        crate::windows_diagnostics::identity_key(&[&process_id, &class_name, &automation_id, &name])
    }
}

fn selection_delta(old: &str, new: &str) -> Option<(&'static str, String)> {
    if old == new {
        return None;
    }
    if old.is_empty() {
        return (!new.is_empty()).then(|| ("selected", new.to_string()));
    }
    if new.is_empty() {
        return Some(("unselected", old.to_string()));
    }

    if let Some(added) = new.strip_suffix(old)
        && !added.is_empty()
    {
        return Some(("selected", added.to_string()));
    }
    if let Some(added) = new.strip_prefix(old)
        && !added.is_empty()
    {
        return Some(("selected", added.to_string()));
    }
    if let Some(removed) = old.strip_suffix(new)
        && !removed.is_empty()
    {
        return Some(("unselected", removed.to_string()));
    }
    if let Some(removed) = old.strip_prefix(new)
        && !removed.is_empty()
    {
        return Some(("unselected", removed.to_string()));
    }

    Some(("selected", new.to_string()))
}

fn update_selection_and_speak(identity: String, new_text: String) {
    let Ok(mut states) = SELECTION_STATES.lock() else {
        return;
    };

    let old_text = if let Some(index) = states.iter().position(|state| state.identity == identity) {
        let previous = states[index].text.clone();
        states[index].text = new_text.clone();
        previous
    } else {
        if states.len() >= MAX_SELECTION_STATES {
            states.remove(0);
        }
        states.push(SelectionState {
            identity,
            text: new_text.clone(),
        });
        String::new()
    };
    drop(states);

    let Some((action, delta)) = selection_delta(&old_text, &new_text) else {
        return;
    };
    if delta.is_empty() {
        return;
    }

    let spoken_delta: String = delta.chars().take(MAX_SELECTION_SPEECH_CHARS).collect();
    if spoken_delta.is_empty() {
        return;
    }

    let sequence = SELECTION_SPEECH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if action == "selected" {
        SELECTION_SELECTED_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        SELECTION_UNSELECTED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    println!(
        "SELECTION_SPEECH #{sequence} | action={action} | text={}",
        crate::windows_diagnostics::text(&spoken_delta)
    );
    crate::windows_speech::speak(&format!("{spoken_delta} {action}"));
}

pub fn print_init_marker() {
    println!("TEXT_PATTERN2_CARET_SELECTION = ENABLED");
    println!("TEXT_PATTERN2_SELECTION_SPEECH = DELTA_ENABLED");
}

pub fn inspect_selection(sequence: u64, element: &IUIAutomationElement) {
    unsafe {
        let process_id = element.CurrentProcessId().unwrap_or_default();
        let is_password: bool = element.CurrentIsPassword().map(Into::into).unwrap_or(false);

        if is_password {
            TEXT_PATTERN2_PROTECTED_COUNT.fetch_add(1, Ordering::Relaxed);
            println!(
                "TEXT_PATTERN2 #{sequence} | PID={process_id} | protected=password | caret_active=<redacted> | selection_ranges=<redacted> | selection_text=<redacted>"
            );
            return;
        }

        let pattern = match element
            .GetCurrentPatternAs::<IUIAutomationTextPattern2>(TEXT_PATTERN2_PATTERN)
        {
            Ok(pattern) => pattern,
            Err(error) => {
                println!(
                    "TEXT_PATTERN2 #{sequence} | PID={process_id} | status=unsupported | error={error}"
                );
                return;
            }
        };

        let mut active = windows_core::BOOL::default();
        if let Err(error) = pattern.GetCaretRange(&mut active) {
            println!(
                "TEXT_PATTERN2 #{sequence} | PID={process_id} | status=caret-error | error={error}"
            );
            return;
        }

        TEXT_PATTERN2_COUNT.fetch_add(1, Ordering::Relaxed);
        if active.as_bool() {
            CARET_ACTIVE_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        let mut selected_parts = Vec::new();
        let selection_ranges = match pattern.GetSelection() {
            Ok(ranges) => {
                let length = ranges.Length().unwrap_or_default().max(0);
                for index in 0..length.min(8) {
                    let Ok(range) = ranges.GetElement(index) else {
                        continue;
                    };
                    let Ok(text) = range.GetText(512) else {
                        continue;
                    };
                    let text = text.display().to_string();
                    if !text.is_empty() {
                        selected_parts.push(text);
                    }
                }
                length
            }
            Err(_) => -1,
        };

        let selected_text = selected_parts.join("\n");
        if !selected_text.is_empty() {
            SELECTION_TEXT_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        let selection_text_log = crate::windows_diagnostics::optional_text(&selected_text);

        println!(
            "TEXT_PATTERN2 #{sequence} | PID={process_id} | status=pass | caret_active={} | selection_ranges={selection_ranges} | selection_text={selection_text_log}",
            active.as_bool()
        );

        let identity = element_identity(element, process_id);
        update_selection_and_speak(identity, selected_text);
    }
}

pub fn print_summary() {
    println!(
        "TEXT_PATTERN2_COUNTS | pass={} | caret_active={} | selection_text={} | protected={}",
        TEXT_PATTERN2_COUNT.load(Ordering::Relaxed),
        CARET_ACTIVE_COUNT.load(Ordering::Relaxed),
        SELECTION_TEXT_COUNT.load(Ordering::Relaxed),
        TEXT_PATTERN2_PROTECTED_COUNT.load(Ordering::Relaxed)
    );
    println!(
        "SELECTION_SPEECH_COUNTS | total={} | selected={} | unselected={}",
        SELECTION_SPEECH_COUNT.load(Ordering::Relaxed),
        SELECTION_SELECTED_COUNT.load(Ordering::Relaxed),
        SELECTION_UNSELECTED_COUNT.load(Ordering::Relaxed)
    );
}
