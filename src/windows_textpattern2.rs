use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::*;
use windows_core::BSTR;

const TEXT_PATTERN2_PATTERN: PATTERNID = PATTERNID(10024);

static TEXT_PATTERN2_COUNT: AtomicU64 = AtomicU64::new(0);
static CARET_ACTIVE_COUNT: AtomicU64 = AtomicU64::new(0);
static SELECTION_TEXT_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_PATTERN2_PROTECTED_COUNT: AtomicU64 = AtomicU64::new(0);

fn one_line(value: BSTR) -> String {
    value
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('|', "\\|")
}

pub fn print_init_marker() {
    println!("TEXT_PATTERN2_CARET_SELECTION = ENABLED");
}

pub fn inspect_selection(sequence: u64, element: &IUIAutomationElement) {
    unsafe {
        let process_id = element.CurrentProcessId().unwrap_or_default();
        let is_password: bool = element
            .CurrentIsPassword()
            .map(Into::into)
            .unwrap_or(false);

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
                    let text = one_line(text);
                    if !text.is_empty() {
                        selected_parts.push(text);
                    }
                }
                length
            }
            Err(_) => -1,
        };

        let selection_text = if selected_parts.is_empty() {
            "<none>".to_string()
        } else {
            SELECTION_TEXT_COUNT.fetch_add(1, Ordering::Relaxed);
            selected_parts.join(" || ")
        };

        println!(
            "TEXT_PATTERN2 #{sequence} | PID={process_id} | status=pass | caret_active={} | selection_ranges={selection_ranges} | selection_text={selection_text}",
            active.as_bool()
        );
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
}
